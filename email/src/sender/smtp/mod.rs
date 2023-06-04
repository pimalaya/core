pub mod config;
pub use config::{SmtpAuthConfig, SmtpConfig};

use mail_parser::{HeaderValue, Message};
use mail_send::{smtp::message as smtp, SmtpClientBuilder};
use std::{collections::HashSet, result, sync::Mutex};
use thiserror::Error;
use tokio::{net::TcpStream, runtime::Runtime};
use tokio_rustls::client::TlsStream;

use crate::{account, email, sender, AccountConfig, EmailHooks, Sender};

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
    ConnectPlainError(#[source] mail_send::Error),
    #[error("cannot connect to smtp server using tls")]
    ConnectError(#[source] mail_send::Error),
    #[error("cannot lock smtp client")]
    LockClientError(String),

    #[error(transparent)]
    SmtpConfigError(#[from] sender::smtp::config::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    EmailError(#[from] email::email::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub enum SmtpClient {
    Tcp(mail_send::SmtpClient<TcpStream>),
    Tls(mail_send::SmtpClient<TlsStream<TcpStream>>),
}

impl SmtpClient {
    pub async fn send<'a>(&mut self, msg: impl smtp::IntoMessage<'a>) -> Result<()> {
        let task = match self {
            Self::Tcp(client) => client.send(msg).await,
            Self::Tls(client) => client.send(msg).await,
        };

        task.map_err(Error::SendError)
    }
}

pub struct Smtp {
    hooks: EmailHooks,
    client: Mutex<SmtpClient>,
    runtime: Runtime,
}

impl Smtp {
    pub fn new(account_config: &AccountConfig, config: &SmtpConfig) -> Result<Self> {
        let hooks = account_config.email_hooks.clone();
        let runtime = Runtime::new().unwrap();

        let mut builder = SmtpClientBuilder::new(config.host.clone(), config.port)
            .implicit_tls(!config.starttls())
            .credentials(config.credentials()?);

        if config.insecure() {
            builder = builder.allow_invalid_certs();
        }

        let client = if config.ssl() {
            SmtpClient::Tls(
                runtime
                    .block_on(builder.connect())
                    .map_err(Error::ConnectError)?,
            )
        } else {
            SmtpClient::Tcp(
                runtime
                    .block_on(builder.connect_plain())
                    .map_err(Error::ConnectPlainError)?,
            )
        };

        Ok(Self {
            hooks,
            client: Mutex::new(client),
            runtime,
        })
    }

    fn into_smtp_msg<'a>(msg: Message<'a>) -> Result<smtp::Message> {
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
            return Err(Error::SendEmailMissingToError);
        }

        Ok(smtp::Message {
            mail_from: mail_from.ok_or(Error::SendEmailMissingFromError)?.into(),
            rcpt_to: rcpt_to
                .into_iter()
                .map(|email| smtp::Address {
                    email: email.into(),
                    parameters: Default::default(),
                })
                .collect(),
            body: msg.raw_message.into(),
        })
    }
}

impl Sender for Smtp {
    fn send(&self, email: &[u8]) -> sender::Result<()> {
        let mut email = Message::parse(&email).ok_or(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.hooks.pre_send.as_ref() {
            buffer = cmd
                .run_with(email.raw_message())
                .map_err(Error::ExecutePreSendHookError)?
                .stdout;
            email = Message::parse(&buffer).ok_or(Error::ParseEmailError)?;
        };

        let email = Self::into_smtp_msg(email)?;
        self.runtime.block_on(
            self.client
                .lock()
                .map_err(|err| Error::LockClientError(err.to_string()))?
                .send(email),
        )?;

        Ok(())
    }
}
