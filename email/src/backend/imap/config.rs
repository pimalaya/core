//! IMAP backend config module.
//!
//! This module contains the representation of the IMAP backend
//! configuration of the user account.

use std::result;
use thiserror::Error;

use crate::process;

#[cfg(feature = "imap-backend")]
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get imap password")]
    GetPasswdError(#[source] process::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,
    #[error("cannot start the notify mode")]
    StartNotifyModeError(#[source] process::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the IMAP backend configuration.
#[cfg(feature = "imap-backend")]
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
    /// Represents the IMAP server password command.
    pub passwd_cmd: String,

    /// Represents the IMAP notify command.
    pub notify_cmd: Option<String>,
    /// Overrides the default IMAP query "NEW" used to fetch new
    /// messages.
    pub notify_query: Option<String>,
    /// Represents the watch commands.
    pub watch_cmds: Option<Vec<String>>,

    /// Represents the OAuth2 config.
    pub oauth2: Option<ImapOauth2Config>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ImapOauth2Config {
    pub method: ImapOauth2Method,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub pkce: bool,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ImapOauth2Method {
    Xoauth2,
    OauthBearer,
}

#[cfg(feature = "imap-backend")]
impl ImapConfig {
    /// Executes the IMAP password command in order to retrieve the
    /// IMAP server password.
    pub fn passwd(&self) -> Result<String> {
        let passwd = process::run(&self.passwd_cmd, &[]).map_err(Error::GetPasswdError)?;
        let passwd = String::from_utf8_lossy(&passwd).to_string();
        let passwd = passwd
            .lines()
            .next()
            .ok_or_else(|| Error::GetPasswdEmptyError)?;
        Ok(passwd.to_owned())
    }

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
