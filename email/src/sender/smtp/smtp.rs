//! SMTP module.
//!
//! This module contains the representation of the SMTP email sender.

use lettre::{
    self,
    address::Envelope,
    error::Error as LettreError,
    transport::smtp::{
        client::{Tls, TlsParameters},
        SmtpTransport,
    },
    Transport,
};
use mailparse::{addrparse_header, MailAddr, MailHeaderMap};
use std::result;
use thiserror::Error;

use crate::{account, email, process, sender, AccountConfig, Sender, SmtpConfig};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build envelope")]
    BuildEnvelopeError(#[source] LettreError),
    #[error("cannot build smtp transport relay")]
    BuildTransportRelayError(#[source] lettre::transport::smtp::Error),
    #[error("cannot build smtp tls parameters")]
    BuildTlsParamsError(#[source] lettre::transport::smtp::Error),
    #[error("cannot parse email before sending")]
    ParseEmailError(#[source] mailparse::MailParseError),
    #[error("cannot send email")]
    SendError(#[source] lettre::transport::smtp::Error),
    #[error("cannot execute pre-send hook")]
    ExecutePreSendHookError(#[source] process::Error),

    #[error(transparent)]
    SmtpConfigError(#[from] sender::smtp::config::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    MsgError(#[from] email::email::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub struct Smtp<'a> {
    account_config: &'a AccountConfig,
    smtp_config: &'a SmtpConfig,
    transport: Option<SmtpTransport>,
}

impl<'a> Smtp<'a> {
    pub fn new(account_config: &'a AccountConfig, smtp_config: &'a SmtpConfig) -> Self {
        Self {
            account_config,
            smtp_config,
            transport: None,
        }
    }

    fn transport(&mut self) -> Result<&SmtpTransport> {
        if let Some(ref transport) = self.transport {
            Ok(transport)
        } else {
            let builder = if self.smtp_config.ssl() {
                let tls = {
                    let builder = TlsParameters::builder(self.smtp_config.host.to_owned())
                        .dangerous_accept_invalid_certs(self.smtp_config.insecure());

                    #[cfg(feature = "native-tls")]
                    let builder =
                        builder.dangerous_accept_invalid_hostnames(self.smtp_config.insecure());

                    builder
                }
                .build()
                .map_err(Error::BuildTlsParamsError)?;

                if self.smtp_config.starttls() {
                    SmtpTransport::starttls_relay(&self.smtp_config.host)
                        .map_err(Error::BuildTransportRelayError)?
                        .tls(Tls::Required(tls))
                } else {
                    SmtpTransport::relay(&self.smtp_config.host)
                        .map_err(Error::BuildTransportRelayError)?
                        .tls(Tls::Wrapper(tls))
                }
            } else {
                SmtpTransport::relay(&self.smtp_config.host)
                    .map_err(Error::BuildTransportRelayError)?
                    .tls(Tls::None)
            };

            self.transport = Some(
                builder
                    .port(self.smtp_config.port)
                    .credentials(self.smtp_config.credentials()?)
                    .build(),
            );

            Ok(self.transport.as_ref().unwrap())
        }
    }
}

impl<'a> Sender for Smtp<'a> {
    fn send(&mut self, email: &[u8]) -> sender::Result<()> {
        let mut email = mailparse::parse_mail(&email).map_err(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.account_config.email_hooks.pre_send.as_deref() {
            buffer = process::run(cmd, email.raw_bytes).map_err(Error::ExecutePreSendHookError)?;
            email = mailparse::parse_mail(&buffer).map_err(Error::ParseEmailError)?;
        };

        let headers = email.get_headers();
        let envelope = Envelope::new(
            headers
                .get_first_header("From")
                .and_then(|header| addrparse_header(header).ok())
                .and_then(|addrs| addrs.into_inner().into_iter().next())
                .and_then(|addr| match addr {
                    MailAddr::Group(group) => {
                        group.addrs.first().and_then(|addr| addr.addr.parse().ok())
                    }
                    MailAddr::Single(single) => single.addr.parse().ok(),
                }),
            headers
                .get_all_headers("To")
                .into_iter()
                .chain(headers.get_all_headers("Cc").into_iter())
                .chain(headers.get_all_headers("Bcc").into_iter())
                .flat_map(addrparse_header)
                .flat_map(|addrs| {
                    addrs
                        .into_inner()
                        .into_iter()
                        .map(|addr| match addr {
                            MailAddr::Group(group) => group.addrs.into_iter().collect::<Vec<_>>(),
                            MailAddr::Single(single) => vec![single],
                        })
                        .collect::<Vec<_>>()
                })
                .flatten()
                .flat_map(|addr| addr.addr.parse())
                .collect::<Vec<_>>(),
        )
        .map_err(Error::BuildEnvelopeError)?;

        // TODO: Bcc should be removed from headers before sending the email.

        self.transport()?
            .send_raw(&envelope, email.raw_bytes)
            .map_err(Error::SendError)?;

        Ok(())
    }
}
