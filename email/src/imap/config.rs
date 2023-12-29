//! Module dedicated to the IMAP backend configuration.
//!
//! This module contains the implementation of the IMAP backend and
//! all associated structures related to it.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    account::config::{oauth2::OAuth2Config, passwd::PasswdConfig},
    Result,
};

/// Errors related to the IMAP backend configuration.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot start the notify mode")]
    StartNotifyModeError(#[source] process::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdError(#[source] secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,
}

/// The IMAP backend configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ImapConfig {
    /// The IMAP server host name.
    pub host: String,

    /// The IMAP server host port.
    pub port: u16,

    /// Enables TLS/SSL.
    ///
    /// Defaults to `true`.
    pub ssl: Option<bool>,

    /// Enables StartTLS.
    ///
    /// Defaults to `false`.
    pub starttls: Option<bool>,

    /// Trusts any certificate.
    ///
    /// Defaults to `false`.
    pub insecure: Option<bool>,

    /// The IMAP server login.
    ///
    /// Usually, the login is either the email address or its left
    /// part (before @).
    pub login: String,

    /// The IMAP server authentication configuration.
    ///
    /// Authentication can be done using password or OAuth 2.0.
    /// See [ImapAuthConfig].
    #[serde(flatten)]
    pub auth: ImapAuthConfig,

    /// The IMAP notify command.
    ///
    /// Defines the command used to notify the user when a new email is available.
    /// Defaults to `notify-send "📫 <sender>" "<subject>"`.
    pub watch: Option<ImapWatchConfig>,
}

impl ImapConfig {
    /// TLS/SSL option getter.
    pub fn ssl(&self) -> bool {
        self.ssl.unwrap_or(true)
    }

    /// StartTLS option getter.
    pub fn starttls(&self) -> bool {
        self.starttls.unwrap_or_default()
    }

    /// Insecure option getter.
    pub fn insecure(&self) -> bool {
        self.insecure.unwrap_or_default()
    }

    /// Builds authentication credentials.
    ///
    /// Authentication credentials can be either a password or an
    /// OAuth 2.0 access token.
    pub async fn build_credentials(&self) -> Result<String> {
        self.auth.build_credentials().await
    }

    /// Find the IMAP watch timeout.
    pub fn find_watch_timeout(&self) -> Option<u64> {
        self.watch.as_ref().and_then(|c| c.find_timeout())
    }
}

/// The IMAP authentication configuration.
///
/// Authentication can be done using password or OAuth 2.0.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "auth")]
pub enum ImapAuthConfig {
    /// The password configuration.
    Passwd(PasswdConfig),

    /// The OAuth 2.0 configuration.
    OAuth2(OAuth2Config),
}

impl Default for ImapAuthConfig {
    fn default() -> Self {
        Self::Passwd(Default::default())
    }
}

impl ImapAuthConfig {
    /// Builds authentication credentials.
    ///
    /// Authentication credentials can be either a password or an
    /// OAuth 2.0 access token.
    pub async fn build_credentials(&self) -> Result<String> {
        match self {
            ImapAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().await.map_err(Error::GetPasswdError)?;
                let passwd = passwd.lines().next().ok_or(Error::GetPasswdEmptyError)?;
                Ok(passwd.to_owned())
            }
            ImapAuthConfig::OAuth2(oauth2) => Ok(oauth2.access_token().await?),
        }
    }

    pub fn replace_undefined_keyring_entries(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();

        match self {
            Self::Passwd(secret) => {
                secret.set_keyring_entry_if_undefined(format!("{name}-imap-passwd"));
            }
            Self::OAuth2(config) => {
                config
                    .client_secret
                    .set_keyring_entry_if_undefined(format!("{name}-imap-oauth2-client-secret"));
                config
                    .access_token
                    .set_keyring_entry_if_undefined(format!("{name}-imap-oauth2-access-token"));
                config
                    .refresh_token
                    .set_keyring_entry_if_undefined(format!("{name}-imap-oauth2-refresh-token"));
            }
        }
    }
}

/// The IMAP watch options (IDLE).
///
/// Options dedicated to the IMAP IDLE mode, which is used to watch
/// changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ImapWatchConfig {
    /// The IMAP watch timeout.
    ///
    /// Timeout used to refresh the IDLE command in
    /// background. Defaults to 29 min as defined in the RFC.
    timeout: Option<u64>,
}

impl ImapWatchConfig {
    /// Find the IMAP watch timeout.
    pub fn find_timeout(&self) -> Option<u64> {
        self.timeout
    }
}
