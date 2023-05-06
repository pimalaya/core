//! IMAP backend config module.
//!
//! This module contains the representation of the IMAP backend
//! configuration of the user account.

use log::debug;
use pimalaya_secret::Secret;
use std::{io, result};
use thiserror::Error;

use crate::{account, process, OAuth2Config, OAuth2Method};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot start the notify mode")]
    StartNotifyModeError(#[source] process::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdError(#[source] pimalaya_secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,

    #[error(transparent)]
    AccountConfigError(#[from] account::config::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the IMAP backend configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImapAuthConfig {
    Passwd(Secret),
    OAuth2(OAuth2Config),
}

impl Default for ImapAuthConfig {
    fn default() -> Self {
        Self::Passwd(Secret::new_raw(""))
    }
}

impl ImapAuthConfig {
    pub fn configure(
        &self,
        reset: bool,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        debug!("configuring imap backend");

        if let ImapAuthConfig::OAuth2(oauth2) = self {
            oauth2.configure(reset, get_client_secret)?;
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
            ImapAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().map_err(Error::GetPasswdError)?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or_else(|| Error::GetPasswdEmptyError)?;
                Ok(Self::Passwd(passwd.to_owned()))
            }
            ImapAuthConfig::OAuth2(config) => Ok(Self::OAuth2AccessToken(
                config.method.clone(),
                config.access_token()?,
            )),
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
