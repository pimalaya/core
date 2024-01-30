pub mod config;

use async_trait::async_trait;
use log::{debug, info};
use mail_parser::{Address, HeaderName, HeaderValue, Message, MessageParser};
use mail_send::{
    smtp::message::{Address as SmtpAddress, IntoMessage, Message as SmtpMessage},
    SmtpClientBuilder,
};
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_rustls::client::TlsStream;

#[cfg(feature = "message-send")]
use crate::message::send::{smtp::SendSmtpMessage, SendMessage};
use crate::{
    account::config::AccountConfig,
    backend::{BackendContext, BackendContextBuilder, BackendFeatureBuilder},
    Result,
};

use self::config::{SmtpAuthConfig, SmtpConfig};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot send email without a sender")]
    SendEmailMissingSenderError,
    #[error("cannot send email without a recipient")]
    SendEmailMissingRecipientError,
    #[error("cannot send email")]
    SendEmailError(#[source] mail_send::Error),
    #[error("cannot connect to smtp server using tcp")]
    ConnectTcpError(#[source] mail_send::Error),
    #[error("cannot connect to smtp server using tls")]
    ConnectTlsError(#[source] mail_send::Error),
}
/// The SMTP backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`SmtpContextSync`].
pub struct SmtpContext {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The SMTP configuration.
    pub smtp_config: Arc<SmtpConfig>,

    /// The SMTP client builder.
    client_builder: mail_send::SmtpClientBuilder<String>,

    /// The SMTP client.
    client: SmtpClientStream,
}

impl SmtpContext {
    pub async fn send(&mut self, msg: &[u8]) -> Result<()> {
        let buffer: Vec<u8>;

        let mut msg = MessageParser::new().parse(msg).unwrap_or_else(|| {
            debug!("cannot parse raw email message");
            Default::default()
        });

        if let Some(cmd) = self.account_config.find_message_pre_send_hook() {
            match cmd.run_with(msg.raw_message()).await {
                Ok(res) => {
                    buffer = res.into();
                    msg = MessageParser::new().parse(&buffer).unwrap_or_else(|| {
                        debug!("cannot parse email raw message");
                        Default::default()
                    });
                }
                Err(err) => {
                    debug!("cannot execute pre-send hook: {err}");
                    debug!("{err:?}");
                }
            }
        };

        match &self.smtp_config.auth {
            SmtpAuthConfig::Passwd(_) => {
                self.client
                    .send(into_smtp_msg(msg)?)
                    .await
                    .map_err(Error::SendEmailError)?;
                Ok(())
            }
            SmtpAuthConfig::OAuth2(oauth2_config) => {
                match self.client.send(into_smtp_msg(msg.clone())?).await {
                    Ok(()) => Ok(()),
                    Err(mail_send::Error::AuthenticationFailed(_)) => {
                        oauth2_config.refresh_access_token().await?;
                        self.client_builder = self
                            .client_builder
                            .clone()
                            .credentials(self.smtp_config.credentials().await?);
                        self.client = if self.smtp_config.is_encryption_enabled() {
                            build_tls_client(&self.client_builder).await
                        } else {
                            build_tcp_client(&self.client_builder).await
                        }?;

                        self.client
                            .send(into_smtp_msg(msg)?)
                            .await
                            .map_err(Error::SendEmailError)?;
                        Ok(())
                    }
                    Err(err) => Err(Error::SendEmailError(err).into()),
                }
            }
        }
    }
}

/// The sync version of the SMTP backend context.
///
/// This is just an SMTP client wrapped into a mutex, so the same SMTP
/// client can be shared and updated across multiple threads.
pub type SmtpContextSync = Arc<Mutex<SmtpContext>>;

impl BackendContext for SmtpContextSync {}

/// The SMTP client builder.
#[derive(Clone)]
pub struct SmtpContextBuilder {
    /// The SMTP configuration.
    smtp_config: Arc<SmtpConfig>,
}

impl SmtpContextBuilder {
    pub fn new(smtp_config: Arc<SmtpConfig>) -> Self {
        Self { smtp_config }
    }
}

#[async_trait]
impl BackendContextBuilder for SmtpContextBuilder {
    type Context = SmtpContextSync;

    #[cfg(feature = "message-send")]
    fn send_message(&self) -> BackendFeatureBuilder<Self::Context, dyn SendMessage> {
        Some(Arc::new(SendSmtpMessage::some_new_boxed))
    }

    /// Build an SMTP sync client.
    ///
    /// The SMTP client is created at this moment. If the client
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new client is created.
    async fn build(self, account_config: Arc<AccountConfig>) -> Result<Self::Context> {
        info!("building new smtp context");

        let mut client_builder =
            SmtpClientBuilder::new(self.smtp_config.host.clone(), self.smtp_config.port)
                .credentials(self.smtp_config.credentials().await?)
                .implicit_tls(!self.smtp_config.is_start_tls_encryption_enabled());

        if self.smtp_config.is_encryption_disabled() {
            client_builder = client_builder.allow_invalid_certs();
        }

        let (client_builder, client) = build_client(&self.smtp_config, client_builder).await?;

        let ctx = SmtpContext {
            account_config,
            smtp_config: self.smtp_config,
            client_builder,
            client,
        };

        Ok(Arc::new(Mutex::new(ctx)))
    }
}

pub enum SmtpClientStream {
    Tcp(mail_send::SmtpClient<TcpStream>),
    Tls(mail_send::SmtpClient<TlsStream<TcpStream>>),
}

impl SmtpClientStream {
    pub async fn send(&mut self, msg: impl IntoMessage<'_>) -> mail_send::Result<()> {
        match self {
            Self::Tcp(client) => client.send(msg).await,
            Self::Tls(client) => client.send(msg).await,
        }
    }
}

pub async fn build_client(
    smtp_config: &SmtpConfig,
    mut client_builder: mail_send::SmtpClientBuilder<String>,
) -> Result<(mail_send::SmtpClientBuilder<String>, SmtpClientStream)> {
    match (&smtp_config.auth, smtp_config.is_encryption_enabled()) {
        (SmtpAuthConfig::Passwd(_), false) => {
            let client = build_tcp_client(&client_builder).await?;
            Ok((client_builder, client))
        }
        (SmtpAuthConfig::Passwd(_), true) => {
            let client = build_tls_client(&client_builder).await?;
            Ok((client_builder, client))
        }
        (SmtpAuthConfig::OAuth2(oauth2_config), false) => {
            match Ok(build_tcp_client(&client_builder).await?) {
                Ok(client) => Ok((client_builder, client)),
                Err(Error::ConnectTcpError(mail_send::Error::AuthenticationFailed(_))) => {
                    oauth2_config.refresh_access_token().await?;
                    client_builder = client_builder.credentials(smtp_config.credentials().await?);
                    let client = build_tcp_client(&client_builder).await?;
                    Ok((client_builder, client))
                }
                Err(err) => Err(err.into()),
            }
        }
        (SmtpAuthConfig::OAuth2(oauth2_config), true) => {
            match Ok(build_tls_client(&client_builder).await?) {
                Ok(client) => Ok((client_builder, client)),
                Err(Error::ConnectTlsError(mail_send::Error::AuthenticationFailed(_))) => {
                    oauth2_config.refresh_access_token().await?;
                    client_builder = client_builder.credentials(smtp_config.credentials().await?);
                    let client = build_tls_client(&client_builder).await?;
                    Ok((client_builder, client))
                }
                Err(err) => Err(err.into()),
            }
        }
    }
}

pub async fn build_tcp_client(
    client_builder: &mail_send::SmtpClientBuilder<String>,
) -> Result<SmtpClientStream> {
    match client_builder.connect_plain().await {
        Ok(client) => Ok(SmtpClientStream::Tcp(client)),
        Err(err) => Err(Error::ConnectTcpError(err).into()),
    }
}

pub async fn build_tls_client(
    client_builder: &mail_send::SmtpClientBuilder<String>,
) -> Result<SmtpClientStream> {
    match client_builder.connect().await {
        Ok(client) => Ok(SmtpClientStream::Tls(client)),
        Err(err) => Err(Error::ConnectTlsError(err).into()),
    }
}

/// Transforms a [`mail_parser::Message`] into a [`mail_send::smtp::message::Message`].
///
/// This function returns an error if no sender or no recipient is
/// found in the original message.
fn into_smtp_msg(msg: Message<'_>) -> Result<SmtpMessage<'_>> {
    let mut mail_from = None;
    let mut rcpt_to = HashSet::new();

    for header in msg.headers() {
        let key = &header.name;
        let val = header.value();

        match key {
            HeaderName::From => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    if let Some(addr) = addrs.first() {
                        if let Some(ref email) = addr.address {
                            mail_from = email.to_string().into();
                        }
                    }
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    if let Some(group) = groups.first() {
                        if let Some(addr) = group.addresses.first() {
                            if let Some(ref email) = addr.address {
                                mail_from = email.to_string().into();
                            }
                        }
                    }
                }
                _ => (),
            },
            HeaderName::To | HeaderName::Cc | HeaderName::Bcc => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    if let Some(addr) = addrs.first() {
                        if let Some(ref email) = addr.address {
                            rcpt_to.insert(email.to_string());
                        }
                    }
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    if let Some(group) = groups.first() {
                        if let Some(addr) = group.addresses.first() {
                            if let Some(ref email) = addr.address {
                                {
                                    rcpt_to.insert(email.to_string());
                                }
                            }
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        };
    }

    if rcpt_to.is_empty() {
        return Err(Error::SendEmailMissingRecipientError.into());
    }

    let msg = SmtpMessage {
        mail_from: mail_from.ok_or(Error::SendEmailMissingSenderError)?.into(),
        rcpt_to: rcpt_to
            .into_iter()
            .map(|email| SmtpAddress {
                email: email.into(),
                ..Default::default()
            })
            .collect(),
        body: msg.raw_message,
    };

    Ok(msg)
}
