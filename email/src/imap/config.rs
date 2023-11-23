//! Module dedicated to the IMAP backend configuration.
//!
//! This module contains the implementation of the IMAP backend and
//! all associated structures related to it.

use process::Cmd;
use thiserror::Error;

use crate::{
    account::{OAuth2Config, PasswdConfig},
    boxed_err, Result,
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
    pub auth: ImapAuthConfig,

    /// The IMAP notify command.
    ///
    /// Defines the command used to notify the user when a new email is available.
    /// Defaults to `notify-send "📫 <sender>" "<subject>"`.
    pub notify_cmd: Option<String>,

    /// The IMAP notify query.
    ///
    /// Defines the IMAP query used to determine the new emails list.
    /// Defaults to `NEW`.
    pub notify_query: Option<String>,

    /// The watch commands.
    ///
    /// Defines the commands to run whenever a change occurs on the
    /// IMAP server.
    pub watch_cmds: Option<Vec<String>>,
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

    /// Notify query option getter.
    pub fn notify_query(&self) -> String {
        self.notify_query
            .as_ref()
            .cloned()
            .unwrap_or_else(|| String::from("NEW"))
    }

    /// Watch commands option getter.
    pub fn watch_cmds(&self) -> Vec<String> {
        self.watch_cmds
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Vec::new())
    }

    /// Runs the IMAP notify command.
    pub async fn run_notify_cmd(
        &self,
        id: u32,
        subject: impl AsRef<str>,
        sender: impl AsRef<str>,
    ) -> Result<()> {
        let cmd = self
            .notify_cmd
            .clone()
            .unwrap_or_else(|| String::from("notify-send \"📫 <sender>\" \"<subject>\""));

        let cmd: Cmd = cmd
            .replace("<id>", &id.to_string())
            .replace("<subject>", subject.as_ref())
            .replace("<sender>", sender.as_ref())
            .into();

        cmd.run()
            .await
            .map_err(|err| boxed_err(Error::StartNotifyModeError(err)))?;

        Ok(())
    }

    /// Builds authentication credentials.
    ///
    /// Authentication credentials can be either a password or an
    /// OAuth 2.0 access token.
    pub async fn build_credentials(&self) -> Result<String> {
        self.auth.build_credentials().await
    }
}

/// The IMAP authentication configuration.
///
/// Authentication can be done using password or OAuth 2.0.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImapAuthConfig {
    /// The password configuration.
    Passwd(PasswdConfig),
    /// The OAuth 2.0 configuration.
    OAuth2(OAuth2Config),
}

impl Default for ImapAuthConfig {
    fn default() -> Self {
        Self::Passwd(PasswdConfig::default())
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
                let passwd = passwd
                    .get()
                    .await
                    .map_err(|err| boxed_err(Error::GetPasswdError(err)))?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or_else(|| boxed_err(Error::GetPasswdEmptyError))?;
                Ok(passwd.to_owned())
            }
            ImapAuthConfig::OAuth2(oauth2) => Ok(oauth2.access_token().await?),
        }
    }
}
