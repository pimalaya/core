//! IMAP backend config module.
//!
//! This module contains the representation of the IMAP backend
//! configuration of the user account.

use log::warn;
use pimalaya_oauth2::AuthorizationCodeGrant;
use pimalaya_secret::Secret;
use std::{io, result, vec};
use thiserror::Error;

use crate::process;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot start the notify mode")]
    StartNotifyModeError(#[source] process::Error),
    #[error("cannot create imap auth without config")]
    BuildImapAuthMissingConfigError,
    #[error("cannot configure imap oauth2")]
    ConfigureOAuth2Error(#[from] pimalaya_oauth2::Error),

    #[error("cannot get imap password from global keyring")]
    GetPasswdError(#[source] pimalaya_secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,

    #[error("cannot get imap oauth2 access token from global keyring")]
    GetOAuth2AccessTokenError(#[source] pimalaya_secret::Error),
    #[error("cannot set imap oauth2 access token")]
    SetOAuth2AccessTokenError(#[source] pimalaya_secret::Error),
    #[error("cannot set imap oauth2 refresh token")]
    SetOAuth2RefreshTokenError(#[source] pimalaya_secret::Error),

    #[error("cannot get imap oauth2 client secret from user")]
    GetOAuth2ClientSecretFromUserError(#[source] io::Error),
    #[error("cannot get imap oauth2 client secret from global keyring")]
    GetOAuth2ClientSecretFromKeyring(#[source] pimalaya_secret::Error),
    #[error("cannot save imap oauth2 client secret into global keyring")]
    SetOAuth2ClientSecretIntoKeyringError(#[source] pimalaya_secret::Error),
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
    Passwd(Secret),
    OAuth2(OAuth2Config),
}

impl ImapAuthConfig {
    pub fn configure(
        &self,
        reset: bool,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        if let ImapAuthConfig::OAuth2(oauth2) = self {
            let set_client_secret = || {
                oauth2
                    .client_secret
                    .set(get_client_secret().map_err(Error::GetOAuth2ClientSecretFromUserError)?)
                    .map_err(Error::SetOAuth2ClientSecretIntoKeyringError)
            };

            let oauth2_client_secret = match oauth2.client_secret.get() {
                _ if reset => set_client_secret(),
                Err(err) if err.is_get_secret_error() => {
                    warn!("cannot find imap oauth2 client secret from keyring, setting it");
                    set_client_secret()
                }
                Err(err) => Err(Error::GetOAuth2ClientSecretFromKeyring(err)),
                Ok(client_secret) => Ok(client_secret),
            }?;

            let mut builder = AuthorizationCodeGrant::new(
                oauth2.client_id.clone(),
                oauth2_client_secret,
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

            oauth2
                .access_token
                .set(access_token)
                .map_err(Error::SetOAuth2AccessTokenError)?;

            if let Some(refresh_token) = &refresh_token {
                oauth2
                    .refresh_token
                    .set(refresh_token)
                    .map_err(Error::SetOAuth2RefreshTokenError)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImapAuth {
    Passwd(String),
    OAuth2AccessToken(OAuth2Method, String),
}

impl ImapAuth {
    pub fn new(config: &ImapAuthConfig) -> Result<Self> {
        match config {
            ImapAuthConfig::None => Err(Error::BuildImapAuthMissingConfigError),
            ImapAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().map_err(Error::GetPasswdError)?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or_else(|| Error::GetPasswdEmptyError)?;
                Ok(Self::Passwd(passwd.to_owned()))
            }
            ImapAuthConfig::OAuth2(config) => {
                let access_token = config
                    .access_token
                    .get()
                    .map_err(Error::GetOAuth2AccessTokenError)?;
                Ok(Self::OAuth2AccessToken(config.method.clone(), access_token))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2Config {
    pub method: OAuth2Method,
    pub client_id: String,
    pub client_secret: Secret,
    pub auth_url: String,
    pub token_url: String,
    pub access_token: Secret,
    pub refresh_token: Secret,
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
