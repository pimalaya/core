use async_trait::async_trait;
use log::{debug, info, warn};
use mail_parser::{Address, HeaderName, HeaderValue, Message, MessageParser};
use mail_send::smtp::message::{Address as SmtpAddress, IntoMessage, Message as SmtpMessage};
use std::{collections::HashSet, ops::Deref, sync::Arc};
use thiserror::Error;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_rustls::client::TlsStream;

use crate::{
    account::AccountConfig,
    backend::BackendContextBuilder,
    sender::{SmtpAuthConfig, SmtpConfig},
    Result,
};

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

/// The SMTP session builder.
#[derive(Clone)]
pub struct SmtpClientBuilder {
    account_config: AccountConfig,
    smtp_config: SmtpConfig,
}

impl SmtpClientBuilder {
    pub async fn new(account_config: AccountConfig, smtp_config: SmtpConfig) -> Self {
        Self {
            account_config,
            smtp_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for SmtpClientBuilder {
    type Context = SmtpClientSync;

    /// Build an SMTP sync session.
    ///
    /// The SMTP session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self) -> Result<Self::Context> {
        info!("building new smtp client");

        let mut client_builder =
            mail_send::SmtpClientBuilder::new(self.smtp_config.host.clone(), self.smtp_config.port)
                .credentials(self.smtp_config.credentials().await?)
                .implicit_tls(!self.smtp_config.starttls());

        if self.smtp_config.insecure() {
            client_builder = client_builder.allow_invalid_certs();
        }

        let (client_builder, client) = build_client(&self.smtp_config, client_builder).await?;

        Ok(SmtpClientSync::new(SmtpClient {
            account_config: self.account_config,
            smtp_config: self.smtp_config,
            client_builder,
            client,
        }))
    }
}

/// The SMTP client.
///
/// This session is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`SmtpSessionSync`].
pub struct SmtpClient {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The SMTP configuration.
    pub smtp_config: SmtpConfig,

    client_builder: mail_send::SmtpClientBuilder<String>,
    client: SmtpClientKind,
}

impl SmtpClient {
    pub async fn send(&mut self, msg: &[u8]) -> Result<()> {
        info!("smtp: sending raw email message");

        let buffer: Vec<u8>;

        let mut msg = MessageParser::new().parse(msg).unwrap_or_else(|| {
            warn!("cannot parse raw message");
            Default::default()
        });

        if let Some(cmd) = self.account_config.email_hooks.pre_send.as_ref() {
            match cmd.run_with(msg.raw_message()).await {
                Ok(res) => {
                    buffer = res.into();
                    msg = MessageParser::new().parse(&buffer).unwrap_or_else(|| {
                        warn!("cannot parse raw message");
                        Default::default()
                    });
                }
                Err(err) => {
                    warn!("cannot execute pre-send hook: {err}");
                    debug!("cannot execute pre-send hook {cmd:?}: {err:?}");
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
                        self.client = if self.smtp_config.ssl() {
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
                    Err(err) => Ok(Err(Error::SendEmailError(err))?),
                }
            }
        }
    }
}

pub enum SmtpClientKind {
    Tcp(mail_send::SmtpClient<TcpStream>),
    Tls(mail_send::SmtpClient<TlsStream<TcpStream>>),
}

impl SmtpClientKind {
    pub async fn send(&mut self, msg: impl IntoMessage<'_>) -> mail_send::Result<()> {
        match self {
            Self::Tcp(client) => client.send(msg).await,
            Self::Tls(client) => client.send(msg).await,
        }
    }
}

/// The sync version of the SMTP session.
///
/// This is just an SMTP session wrapped into a mutex, so the same
/// SMTP session can be shared and updated across multiple threads.
#[derive(Clone)]
pub struct SmtpClientSync(Arc<Mutex<SmtpClient>>);

impl SmtpClientSync {
    /// Create a new SMTP sync session from an SMTP session.
    pub fn new(client: SmtpClient) -> Self {
        Self(Arc::new(Mutex::new(client)))
    }
}

impl Deref for SmtpClientSync {
    type Target = Mutex<SmtpClient>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn build_client(
    smtp_config: &SmtpConfig,
    mut client_builder: mail_send::SmtpClientBuilder<String>,
) -> Result<(mail_send::SmtpClientBuilder<String>, SmtpClientKind)> {
    match (&smtp_config.auth, smtp_config.ssl()) {
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
                Err(err) => Ok(Err(err)?),
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
                Err(err) => Ok(Err(err)?),
            }
        }
    }
}

pub async fn build_tcp_client(
    client_builder: &mail_send::SmtpClientBuilder<String>,
) -> Result<SmtpClientKind> {
    match client_builder.connect_plain().await {
        Ok(client) => Ok(SmtpClientKind::Tcp(client)),
        Err(err) => Ok(Err(Error::ConnectTcpError(err))?),
    }
}

pub async fn build_tls_client(
    client_builder: &mail_send::SmtpClientBuilder<String>,
) -> Result<SmtpClientKind> {
    match client_builder.connect().await {
        Ok(client) => Ok(SmtpClientKind::Tls(client)),
        Err(err) => Ok(Err(Error::ConnectTlsError(err))?),
    }
}

/// Transforms a [`mail_parser::Message`] into a [`mail_send::smtp::message::Message`].
///
/// This function returns an error if no sender or no recipient is
/// found in the original message.
fn into_smtp_msg<'a>(msg: Message<'a>) -> Result<SmtpMessage<'a>> {
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
                        if let Some(ref addr) = group.addresses.first() {
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
                        if let Some(ref addr) = group.addresses.first() {
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
        return Ok(Err(Error::SendEmailMissingRecipientError)?);
    }

    let msg = SmtpMessage {
        mail_from: mail_from.ok_or(Error::SendEmailMissingSenderError)?.into(),
        rcpt_to: rcpt_to
            .into_iter()
            .map(|email| SmtpAddress {
                email: email.into(),
                parameters: Default::default(),
            })
            .collect(),
        body: msg.raw_message.into(),
    };

    Ok(msg)
}
