pub mod config;
mod error;

use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use futures::lock::Mutex;
use mail_parser::{Addr, Address, HeaderName, HeaderValue, Message, MessageParser};
use mail_send::{
    smtp::message::{Address as SmtpAddress, IntoMessage, Message as SmtpMessage},
    SmtpClientBuilder,
};
#[cfg(feature = "tokio")]
use tokio::net::TcpStream;
#[cfg(feature = "tokio-native-tls")]
use tokio_native_tls::TlsStream;
#[cfg(feature = "tokio-rustls")]
use tokio_rustls::client::TlsStream;
use tracing::{debug, info, warn};

use self::config::{SmtpAuthConfig, SmtpConfig};
#[doc(inline)]
pub use self::error::{Error, Result};
use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    message::send::{smtp::SendSmtpMessage, SendMessage},
    retry::{Retry, RetryState},
    AnyResult,
};

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
                Err(_err) => {
                    debug!("cannot execute pre-send hook: {_err}");
                    debug!("{_err:?}");
                }
            }
        };

        let mut retry = Retry::default();

        loop {
            // NOTE: cannot clone the final message
            let msg = into_smtp_msg(msg.clone())?;

            match retry.next(retry.timeout(self.client.send(msg)).await) {
                RetryState::Retry => {
                    debug!(attempt = retry.attempts, "request timed out");
                    continue;
                }
                RetryState::TimedOut => {
                    break Err(Error::SendMessageTimedOutError);
                }
                RetryState::Ok(Ok(res)) => {
                    break Ok(res);
                }
                RetryState::Ok(Err(err)) => {
                    match err {
                        mail_send::Error::Timeout => {
                            warn!("connection timed out");
                        }
                        mail_send::Error::Io(err) => {
                            let reason = err.to_string();
                            warn!(reason, "connection broke");
                        }
                        mail_send::Error::UnexpectedReply(reply) => {
                            let reason = reply.message;
                            let code = reply.code;
                            warn!(reason, "server replied with code {code}");
                        }
                        err => {
                            break Err(Error::SendMessageError(err));
                        }
                    };

                    debug!("re-connecting…");

                    self.client = if self.smtp_config.is_encryption_enabled() {
                        build_tls_client(&self.client_builder).await
                    } else {
                        build_tcp_client(&self.client_builder).await
                    }?;

                    retry.reset();
                    continue;
                }
            }
        }
    }

    pub async fn noop(&mut self) -> Result<()> {
        self.client.noop().await
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
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The SMTP configuration.
    smtp_config: Arc<SmtpConfig>,
}

impl SmtpContextBuilder {
    pub fn new(account_config: Arc<AccountConfig>, smtp_config: Arc<SmtpConfig>) -> Self {
        Self {
            account_config,
            smtp_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for SmtpContextBuilder {
    type Context = SmtpContextSync;

    fn check_up(&self) -> Option<BackendFeature<Self::Context, dyn CheckUp>> {
        Some(Arc::new(CheckUpSmtp::some_new_boxed))
    }

    fn send_message(&self) -> Option<BackendFeature<Self::Context, dyn SendMessage>> {
        Some(Arc::new(SendSmtpMessage::some_new_boxed))
    }

    /// Build an SMTP sync client.
    ///
    /// The SMTP client is created at this moment. If the client
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new client is created.
    async fn build(self) -> AnyResult<Self::Context> {
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
            account_config: self.account_config,
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

    pub async fn noop(&mut self) -> Result<()> {
        match self {
            Self::Tcp(client) => client.noop().await.map_err(Error::MailSendNoOpFailed),
            Self::Tls(client) => client.noop().await.map_err(Error::MailSendNoOpFailed),
        }
    }
}

#[derive(Clone)]
pub struct CheckUpSmtp {
    ctx: SmtpContextSync,
}

impl CheckUpSmtp {
    pub fn new(ctx: &SmtpContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &SmtpContextSync) -> Box<dyn CheckUp> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &SmtpContextSync) -> Option<Box<dyn CheckUp>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CheckUp for CheckUpSmtp {
    async fn check_up(&self) -> AnyResult<()> {
        let mut ctx = self.ctx.lock().await;
        Ok(ctx.noop().await?)
    }
}

pub async fn build_client(
    smtp_config: &SmtpConfig,
    #[cfg_attr(not(feature = "oauth2"), allow(unused_mut))]
    mut client_builder: mail_send::SmtpClientBuilder<String>,
) -> Result<(mail_send::SmtpClientBuilder<String>, SmtpClientStream)> {
    match (&smtp_config.auth, smtp_config.is_encryption_enabled()) {
        (SmtpAuthConfig::Password(_), false) => {
            let client = build_tcp_client(&client_builder).await?;
            Ok((client_builder, client))
        }
        (SmtpAuthConfig::Password(_), true) => {
            let client = build_tls_client(&client_builder).await?;
            Ok((client_builder, client))
        }
        #[cfg(feature = "oauth2")]
        (SmtpAuthConfig::OAuth2(oauth2_config), false) => {
            match Ok(build_tcp_client(&client_builder).await?) {
                Ok(client) => Ok((client_builder, client)),
                Err(Error::ConnectTcpSmtpError(mail_send::Error::AuthenticationFailed(_))) => {
                    warn!("authentication failed, refreshing access token and retrying…");
                    oauth2_config
                        .refresh_access_token()
                        .await
                        .map_err(|_| Error::RefreshingAccessTokenFailed)?;
                    client_builder = client_builder.credentials(smtp_config.credentials().await?);
                    let client = build_tcp_client(&client_builder).await?;
                    Ok((client_builder, client))
                }
                Err(err) => Err(err),
            }
        }
        #[cfg(feature = "oauth2")]
        (SmtpAuthConfig::OAuth2(oauth2_config), true) => {
            match Ok(build_tls_client(&client_builder).await?) {
                Ok(client) => Ok((client_builder, client)),
                Err(Error::ConnectTlsSmtpError(mail_send::Error::AuthenticationFailed(_))) => {
                    warn!("authentication failed, refreshing access token and retrying…");
                    oauth2_config
                        .refresh_access_token()
                        .await
                        .map_err(|_| Error::RefreshingAccessTokenFailed)?;
                    client_builder = client_builder.credentials(smtp_config.credentials().await?);
                    let client = build_tls_client(&client_builder).await?;
                    Ok((client_builder, client))
                }
                Err(err) => Err(err),
            }
        }
    }
}

pub async fn build_tcp_client(
    client_builder: &mail_send::SmtpClientBuilder<String>,
) -> Result<SmtpClientStream> {
    match client_builder.connect_plain().await {
        Ok(client) => Ok(SmtpClientStream::Tcp(client)),
        Err(err) => Err(Error::ConnectTcpSmtpError(err)),
    }
}

pub async fn build_tls_client(
    client_builder: &mail_send::SmtpClientBuilder<String>,
) -> Result<SmtpClientStream> {
    match client_builder.connect().await {
        Ok(client) => Ok(SmtpClientStream::Tls(client)),
        Err(err) => Err(Error::ConnectTlsSmtpError(err)),
    }
}

/// Transform a [`mail_parser::Message`] into a
/// [`mail_send::smtp::message::Message`].
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
                    if let Some(email) = addrs.first().and_then(find_valid_email) {
                        mail_from = email.to_string().into();
                    }
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    if let Some(group) = groups.first() {
                        if let Some(email) = group.addresses.first().and_then(find_valid_email) {
                            mail_from = email.to_string().into();
                        }
                    }
                }
                _ => (),
            },
            HeaderName::To | HeaderName::Cc | HeaderName::Bcc => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    rcpt_to.extend(addrs.iter().filter_map(find_valid_email));
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    rcpt_to.extend(
                        groups
                            .iter()
                            .flat_map(|group| group.addresses.iter())
                            .filter_map(find_valid_email),
                    );
                }
                _ => (),
            },
            _ => (),
        };
    }

    if rcpt_to.is_empty() {
        return Err(Error::SendMessageMissingRecipientError);
    }

    let msg = SmtpMessage {
        mail_from: mail_from
            .ok_or(Error::SendMessageMissingSenderError)?
            .into(),
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

fn find_valid_email(addr: &Addr) -> Option<String> {
    match &addr.address {
        None => None,
        Some(email) => {
            let email = email.trim();
            if email.is_empty() {
                None
            } else {
                Some(email.to_string())
            }
        }
    }
}
