//! Module dedicated to the SMTP sender.
//!
//! This module contains the implementation of the SMTP sender and all
//! associated structures related to it.

pub mod config;

use async_trait::async_trait;
use futures::executor::block_on;
use log::error;
use mail_parser::{HeaderValue, Message};
use mail_send::{smtp::message as smtp, SmtpClientBuilder};
use std::collections::HashSet;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use crate::{account::AccountConfig, sender::Sender, Result};

#[doc(inline)]
pub use self::config::{SmtpAuthConfig, SmtpConfig};

/// Errors related to the SMTP sender.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse email before sending")]
    ParseEmailError,
    #[error("cannot execute pre-send hook")]
    ExecutePreSendHookError(#[source] pimalaya_process::Error),
    #[error("cannot send email")]
    SendError(#[source] mail_send::Error),
    #[error("cannot send email: missing sender")]
    SendEmailMissingFromError,
    #[error("cannot send email: missing recipient")]
    SendEmailMissingToError,
    #[error("cannot connect to smtp server")]
    ConnectTcpError(#[source] mail_send::Error),
    #[error("cannot connect to smtp server using tls")]
    ConnectTlsError(#[source] mail_send::Error),
}

enum SmtpClient {
    Tcp(mail_send::SmtpClient<TcpStream>),
    Tls(mail_send::SmtpClient<TlsStream<TcpStream>>),
}

impl SmtpClient {
    pub async fn send<'a>(&mut self, msg: impl smtp::IntoMessage<'a>) -> mail_send::Result<()> {
        match self {
            Self::Tcp(client) => client.send(msg).await,
            Self::Tls(client) => client.send(msg).await,
        }
    }
}

/// The SMTP sender.
pub struct Smtp {
    account_config: AccountConfig,
    smtp_config: SmtpConfig,
    client_builder: SmtpClientBuilder<String>,
    client: SmtpClient,
}

impl Smtp {
    /// Creates a new SMTP sender from configurations.
    pub fn new(account_config: AccountConfig, smtp_config: SmtpConfig) -> Result<Self> {
        let mut client_builder = SmtpClientBuilder::new(smtp_config.host.clone(), smtp_config.port)
            .credentials(smtp_config.credentials()?)
            .implicit_tls(!smtp_config.starttls());

        if smtp_config.insecure() {
            client_builder = client_builder.allow_invalid_certs();
        }

        let (client_builder, client) = block_on(Self::build_client(&smtp_config, client_builder))?;

        Ok(Self {
            account_config,
            smtp_config,
            client_builder,
            client,
        })
    }

    async fn build_client(
        smtp_config: &SmtpConfig,
        mut client_builder: SmtpClientBuilder<String>,
    ) -> Result<(SmtpClientBuilder<String>, SmtpClient)> {
        match (&smtp_config.auth, smtp_config.ssl()) {
            (SmtpAuthConfig::Passwd(_), false) => {
                let client = Self::build_tcp_client(&client_builder).await?;
                Ok((client_builder, client))
            }
            (SmtpAuthConfig::Passwd(_), true) => {
                let client = Self::build_tls_client(&client_builder).await?;
                Ok((client_builder, client))
            }
            (SmtpAuthConfig::OAuth2(oauth2_config), false) => {
                match Ok(Self::build_tcp_client(&client_builder).await?) {
                    Ok(client) => Ok((client_builder, client)),
                    Err(Error::ConnectTcpError(mail_send::Error::AuthenticationFailed(_))) => {
                        oauth2_config.refresh_access_token()?;
                        client_builder = client_builder.credentials(smtp_config.credentials()?);
                        let client = Self::build_tcp_client(&client_builder).await?;
                        Ok((client_builder, client))
                    }
                    Err(err) => {
                        error!("{err:?}");
                        Ok(Err(err)?)
                    }
                }
            }
            (SmtpAuthConfig::OAuth2(oauth2_config), true) => {
                match Ok(Self::build_tls_client(&client_builder).await?) {
                    Ok(client) => Ok((client_builder, client)),
                    Err(Error::ConnectTlsError(mail_send::Error::AuthenticationFailed(_))) => {
                        oauth2_config.refresh_access_token()?;
                        client_builder = client_builder.credentials(smtp_config.credentials()?);
                        let client = Self::build_tls_client(&client_builder).await?;
                        Ok((client_builder, client))
                    }
                    Err(err) => Ok(Err(err)?),
                }
            }
        }
    }

    async fn build_tcp_client(client_builder: &SmtpClientBuilder<String>) -> Result<SmtpClient> {
        match client_builder.connect_plain().await {
            Ok(client) => Ok(SmtpClient::Tcp(client)),
            Err(err) => Ok(Err(Error::ConnectTcpError(err))?),
        }
    }

    async fn build_tls_client(client_builder: &SmtpClientBuilder<String>) -> Result<SmtpClient> {
        match client_builder.connect().await {
            Ok(client) => Ok(SmtpClient::Tls(client)),
            Err(err) => Ok(Err(Error::ConnectTlsError(err))?),
        }
    }

    async fn send(&mut self, email: &[u8]) -> Result<()> {
        let mut email = Message::parse(&email).ok_or(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.account_config.email_hooks.pre_send.as_ref() {
            buffer = cmd
                .run_with(email.raw_message())
                .map_err(Error::ExecutePreSendHookError)?
                .stdout;
            email = Message::parse(&buffer).ok_or(Error::ParseEmailError)?;
        };

        match &self.smtp_config.auth {
            SmtpAuthConfig::Passwd(_) => {
                self.client
                    .send(into_smtp_msg(email)?)
                    .await
                    .map_err(Error::SendError)?;
                Ok(())
            }
            SmtpAuthConfig::OAuth2(oauth2_config) => {
                match self.client.send(into_smtp_msg(email.clone())?).await {
                    Ok(()) => Ok(()),
                    Err(mail_send::Error::AuthenticationFailed(_)) => {
                        oauth2_config.refresh_access_token()?;
                        self.client_builder = self
                            .client_builder
                            .clone()
                            .credentials(self.smtp_config.credentials()?);
                        self.client = if self.smtp_config.ssl() {
                            Self::build_tls_client(&self.client_builder).await
                        } else {
                            Self::build_tcp_client(&self.client_builder).await
                        }?;

                        self.client
                            .send(into_smtp_msg(email)?)
                            .await
                            .map_err(Error::SendError)?;
                        Ok(())
                    }
                    Err(err) => Ok(Err(Error::SendError(err))?),
                }
            }
        }
    }
}

#[async_trait]
impl Sender for Smtp {
    async fn send(&mut self, email: &[u8]) -> Result<()> {
        Ok(self.send(email).await?)
    }
}

fn into_smtp_msg<'a>(msg: Message<'a>) -> Result<smtp::Message<'a>> {
    let mut mail_from = None;
    let mut rcpt_to = HashSet::new();

    for header in msg.headers() {
        let key = header.name();
        let val = header.value();

        if key.eq_ignore_ascii_case("from") {
            if let HeaderValue::Address(addr) = val {
                if let Some(email) = &addr.address {
                    mail_from = email.to_string().into();
                }
            }
        } else if key.eq_ignore_ascii_case("to")
            || key.eq_ignore_ascii_case("cc")
            || key.eq_ignore_ascii_case("bcc")
        {
            match val {
                HeaderValue::Address(addr) => {
                    if let Some(email) = &addr.address {
                        rcpt_to.insert(email.to_string());
                    }
                }
                HeaderValue::AddressList(addrs) => {
                    for addr in addrs {
                        if let Some(email) = &addr.address {
                            rcpt_to.insert(email.to_string());
                        }
                    }
                }
                HeaderValue::Group(group) => {
                    for addr in &group.addresses {
                        if let Some(email) = &addr.address {
                            rcpt_to.insert(email.to_string());
                        }
                    }
                }
                HeaderValue::GroupList(groups) => {
                    for group in groups {
                        for addr in &group.addresses {
                            if let Some(email) = &addr.address {
                                rcpt_to.insert(email.to_string());
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    if rcpt_to.is_empty() {
        return Ok(Err(Error::SendEmailMissingToError)?);
    }

    let msg = smtp::Message {
        mail_from: mail_from.ok_or(Error::SendEmailMissingFromError)?.into(),
        rcpt_to: rcpt_to
            .into_iter()
            .map(|email| smtp::Address {
                email: email.into(),
                parameters: Default::default(),
            })
            .collect(),
        body: msg.raw_message.into(),
    };

    Ok(msg)
}
