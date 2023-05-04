//! IMAP backend config module.
//!
//! This module contains the representation of the IMAP backend
//! configuration of the user account.

use keyring::Entry;
use pimalaya_oauth2::AuthorizationCodeGrant;
use std::{fmt, io, result, vec};
use thiserror::Error;

use crate::process;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get imap password")]
    GetPasswdError(#[source] process::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,
    #[error("cannot start the notify mode")]
    StartNotifyModeError(#[source] process::Error),
    #[error("cannot create imap auth without config")]
    BuildImapAuthMissingConfigError,
    #[error("cannot get imap oauth2 credentials using global keyring")]
    BuildImapAuthKeyringError(#[from] keyring::Error),
    #[error("cannot configure imap oauth2")]
    ConfigureOAuth2Error(#[from] pimalaya_oauth2::Error),
    #[error("cannot get oauth2 imap client secret from global keyring")]
    GetOAuth2ImapClientSecretFromKeyring(#[source] keyring::Error),
    #[error("cannot get oauth2 imap client secret from user")]
    GetOAuth2ImapClientSecretFromUserError(#[source] io::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the IMAP backend configuration.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct ImapConfig {
    /// Represents the IMAP server host.
    pub host: String,
    /// Represents the IMAP server port.
    pub port: u16,
    /// Enables SSL.
    pub ssl: Option<bool>,
    /// Enables StartTLS.
    pub starttls: Option<bool>,
    /// Trusts any certificate.
    pub insecure: Option<bool>,
    /// Represents the IMAP server login.
    pub login: String,
    /// Represents the IMAP server authentication configuration.
    pub auth: ImapAuthConfig,

    /// Represents the IMAP notify command.
    pub notify_cmd: Option<String>,
    /// Overrides the default IMAP query "NEW" used to fetch new
    /// messages.
    pub notify_query: Option<String>,
    /// Represents the watch commands.
    pub watch_cmds: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ImapAuthConfig {
    #[default]
    None,
    RawPasswd(String),
    PasswdCmd(String),
    OAuth2(OAuth2Config),
}

impl ImapAuthConfig {
    pub fn configure<N>(
        &self,
        name: N,
        reset: bool,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()>
    where
        N: fmt::Display,
    {
        if let ImapAuthConfig::OAuth2(oauth2) = self {
            let mut builder = AuthorizationCodeGrant::new(
                oauth2.client_id.clone(),
                match &oauth2.client_secret {
                    OAuth2ClientSecret::Keyring => {
                        let entry = Entry::new(
                            "pimalaya-email",
                            &format!("oauth2-imap-client-secret-{name}"),
                        )?;

                        let set_client_secret = || -> Result<String> {
                            let secret = get_client_secret()
                                .map_err(Error::GetOAuth2ImapClientSecretFromUserError)?;
                            entry.set_password(&secret)?;
                            Ok(secret)
                        };

                        match entry.get_password() {
                            _ if reset => set_client_secret()?,
                            Err(_) => set_client_secret()?,
                            Ok(secret) => secret,
                        }
                    }
                    OAuth2ClientSecret::Cmd(cmd) => {
                        let passwd = process::run(&cmd, &[]).map_err(Error::GetPasswdError)?;
                        let passwd = String::from_utf8_lossy(&passwd).to_string();
                        let passwd = passwd
                            .lines()
                            .next()
                            .ok_or_else(|| Error::GetPasswdEmptyError)?;
                        passwd.to_owned()
                    }
                    OAuth2ClientSecret::Raw(secret) => secret.to_owned(),
                },
                oauth2.auth_url.clone(),
                oauth2.token_url.clone(),
            )?;

            if oauth2.pkce {
                builder = builder.with_pkce();
            }

            for scope in oauth2.scopes.clone() {
                builder = builder.with_scope(scope);
            }

            let client = builder.get_client()?;
            let (redirect_url, csrf_token) = builder.get_redirect_url(&client);

            println!("To enable OAuth2, click on the following link:");
            println!("");
            println!("{}", redirect_url.to_string());

            let (access_token, refresh_token) = builder.wait_for_redirection(client, csrf_token)?;

            Entry::new(
                "pimalaya-email",
                &format!("oauth2-imap-access-token-{name}"),
            )?
            .set_password(&access_token)?;

            if let Some(refresh_token) = &refresh_token {
                Entry::new(
                    "pimalaya-email",
                    &format!("oauth2-imap-refresh-token-{name}"),
                )?
                .set_password(refresh_token)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImapAuth {
    Passwd(String),
    AccessToken(OAuth2Method, String),
}

impl ImapAuth {
    pub fn new<N>(name: N, config: ImapAuthConfig) -> Result<Self>
    where
        N: fmt::Display,
    {
        match config {
            ImapAuthConfig::None => Err(Error::BuildImapAuthMissingConfigError),
            ImapAuthConfig::RawPasswd(passwd) => Ok(Self::Passwd(passwd)),
            ImapAuthConfig::PasswdCmd(cmd) => {
                let passwd = process::run(&cmd, &[]).map_err(Error::GetPasswdError)?;
                let passwd = String::from_utf8_lossy(&passwd).to_string();
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or_else(|| Error::GetPasswdEmptyError)?;
                Ok(Self::Passwd(passwd.to_owned()))
            }
            ImapAuthConfig::OAuth2(config) => {
                let access_token = Entry::new(
                    "pimalaya-email",
                    &format!("oauth2-imap-access-token-{name}"),
                )?
                .get_password()
                .unwrap_or_default();
                Ok(Self::AccessToken(config.method, access_token))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2Config {
    pub method: OAuth2Method,
    pub client_id: String,
    pub client_secret: OAuth2ClientSecret,
    pub auth_url: String,
    pub token_url: String,
    pub pkce: bool,
    pub scopes: OAuth2Scopes,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum OAuth2Method {
    #[default]
    XOAuth2,
    OAuthBearer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OAuth2ClientSecret {
    Keyring,
    Cmd(String),
    Raw(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OAuth2Scopes {
    Scope(String),
    Scopes(Vec<String>),
}

impl IntoIterator for OAuth2Scopes {
    type Item = String;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Scope(scope) => vec![scope].into_iter(),
            Self::Scopes(scopes) => scopes.into_iter(),
        }
    }
}

impl ImapConfig {
    /// Gets the SSL IMAP option.
    pub fn ssl(&self) -> bool {
        self.ssl.unwrap_or(true)
    }

    /// Gets the StartTLS IMAP option.
    pub fn starttls(&self) -> bool {
        self.starttls.unwrap_or_default()
    }

    /// Gets the StartTLS IMAP option.
    pub fn insecure(&self) -> bool {
        self.insecure.unwrap_or_default()
    }

    /// Runs the IMAP notify command.
    pub fn run_notify_cmd<S: AsRef<str>>(&self, id: u32, subject: S, sender: S) -> Result<()> {
        let mut cmd = self
            .notify_cmd
            .clone()
            .unwrap_or_else(|| String::from("notify-send \"📫 <sender>\" \"<subject>\""));

        cmd = cmd
            .replace("<id>", &id.to_string())
            .replace("<subject>", subject.as_ref())
            .replace("<sender>", sender.as_ref());

        process::run(&cmd, &[]).map_err(Error::StartNotifyModeError)?;

        Ok(())
    }

    pub fn notify_query(&self) -> String {
        self.notify_query
            .as_ref()
            .cloned()
            .unwrap_or_else(|| String::from("NEW"))
    }

    pub fn watch_cmds(&self) -> Vec<String> {
        self.watch_cmds
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Vec::new())
    }
}
