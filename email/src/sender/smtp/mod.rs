pub mod config;

use mail_parser::{HeaderValue, Message};
use mail_send::{smtp::message as smtp, SmtpClientBuilder};
use std::{collections::HashSet, io, result};
use thiserror::Error;
use tokio::{net::TcpStream, runtime::Runtime};
use tokio_rustls::client::TlsStream;

use crate::{account, email, sender, AccountConfig, EmailHooks, Sender};
pub use config::{SmtpAuthConfig, SmtpConfig};

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
    #[error("cannot get async runtime")]
    GetAsyncRuntimeError(#[source] io::Error),

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
    pub async fn send<'a>(&mut self, msg: impl smtp::IntoMessage<'a>) -> mail_send::Result<()> {
        match self {
            Self::Tcp(client) => client.send(msg).await,
            Self::Tls(client) => client.send(msg).await,
        }
    }
}

pub struct Smtp {
    config: SmtpConfig,
    hooks: EmailHooks,
    runtime: Runtime,
    client_builder: SmtpClientBuilder<String>,
    client: SmtpClient,
}

impl Smtp {
    pub fn new(account_config: &AccountConfig, smtp_config: &SmtpConfig) -> Result<Self> {
        let config = smtp_config.to_owned();
        let hooks = account_config.email_hooks.clone();
        let runtime = Runtime::new().map_err(Error::GetAsyncRuntimeError)?;

        let mut client_builder = SmtpClientBuilder::new(smtp_config.host.clone(), smtp_config.port)
            .implicit_tls(!smtp_config.starttls());

        if smtp_config.insecure() {
            client_builder = client_builder.allow_invalid_certs();
        }

        let (client_builder, client) =
            Self::get_client_with_builder(smtp_config, &runtime, client_builder)?;

        Ok(Self {
            config,
            hooks,
            runtime,
            client_builder,
            client,
        })
    }

    fn get_client_with_builder(
        smtp_config: &SmtpConfig,
        runtime: &Runtime,
        mut client_builder: SmtpClientBuilder<String>,
    ) -> Result<(SmtpClientBuilder<String>, SmtpClient)> {
        client_builder = client_builder.credentials(smtp_config.credentials()?);

        let client = if smtp_config.ssl() {
            SmtpClient::Tls(
                runtime
                    .block_on(client_builder.connect())
                    .map_err(Error::ConnectError)?,
            )
        } else {
            SmtpClient::Tcp(
                runtime
                    .block_on(client_builder.connect_plain())
                    .map_err(Error::ConnectPlainError)?,
            )
        };

        Ok((client_builder, client))
    }

    fn block_on_send(&mut self, email: Message) -> Result<()> {
        self.runtime
            .block_on(self.client.send(into_smtp_msg(email)?))
            .map_err(Error::SendError)
    }

    fn send(&mut self, email: &[u8]) -> Result<()> {
        let mut email = Message::parse(&email).ok_or(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.hooks.pre_send.as_ref() {
            buffer = cmd
                .run_with(email.raw_message())
                .map_err(Error::ExecutePreSendHookError)?
                .stdout;
            email = Message::parse(&buffer).ok_or(Error::ParseEmailError)?;
        };

        match self.config.auth.clone() {
            SmtpAuthConfig::Passwd(_) => self.block_on_send(email),
            SmtpAuthConfig::OAuth2(oauth2) => {
                let client_builder = self.client_builder.clone();
                self.block_on_send(email.clone()).or_else(|err| match err {
                    Error::SendError(mail_send::Error::AuthenticationFailed(_)) => {
                        oauth2.refresh_access_token()?;

                        let (client_builder, client) = Self::get_client_with_builder(
                            &self.config,
                            &self.runtime,
                            client_builder.credentials(self.config.credentials()?),
                        )?;

                        self.client_builder = client_builder;
                        self.client = client;
                        self.block_on_send(email)
                    }
                    err => Err(err),
                })
            }
        }
    }
}

impl Sender for Smtp {
    fn send(&mut self, email: &[u8]) -> sender::Result<()> {
        Ok(self.send(email)?)
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
        return Err(Error::SendEmailMissingToError);
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
