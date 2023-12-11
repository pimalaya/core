//! Module dedicated to the SMTP sender configuration.
//!
//! This module contains the configuration specific to the SMTP
//! sender.

use log::debug;
use mail_send::Credentials;
use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

use crate::{
    account::config::{
        oauth2::{OAuth2Config, OAuth2Method},
        passwd::PasswdConfig,
    },
    Result,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get smtp password")]
    GetPasswdError(#[source] secret::Error),
    #[error("cannot get smtp password: password is empty")]
    GetPasswdEmptyError,
}

/// The SMTP sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SmtpConfig {
    /// The SMTP server host name.
    pub host: String,

    /// The SMTP server host port.
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

    /// The SMTP server login.
    ///
    /// Usually, the login is either the email address or its left
    /// part (before @).
    pub login: String,

    /// The SMTP server authentication configuration.
    ///
    /// Authentication can be done using password or OAuth 2.0.
    /// See [SmtpAuthConfig].
    #[serde(flatten)]
    pub auth: SmtpAuthConfig,
}

impl SmtpConfig {
    /// Builds the SMTP credentials string.
    ///
    /// The result depends on the [`SmtpAuthConfig`]: if password mode
    /// then creates credentials from login/password, if OAuth 2.0
    /// then creates credentials from access token.
    pub async fn credentials(&self) -> Result<Credentials<String>> {
        Ok(match &self.auth {
            SmtpAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().await.map_err(Error::GetPasswdError)?;
                let passwd = passwd.lines().next().ok_or(Error::GetPasswdEmptyError)?;
                Credentials::new(self.login.clone(), passwd.to_owned())
            }
            SmtpAuthConfig::OAuth2(oauth2) => match oauth2.method {
                OAuth2Method::XOAuth2 => {
                    Credentials::new_xoauth2(self.login.clone(), oauth2.access_token().await?)
                }
                OAuth2Method::OAuthBearer => Credentials::new_oauth(oauth2.access_token().await?),
            },
        })
    }

    /// SSL enabled getter.
    pub fn ssl(&self) -> bool {
        self.ssl.unwrap_or(true)
    }

    /// STARTTLS enabled getter.
    pub fn starttls(&self) -> bool {
        self.starttls.unwrap_or_default()
    }

    /// Insecure mode getter
    pub fn insecure(&self) -> bool {
        self.insecure.unwrap_or_default()
    }
}

/// The SMTP authentication configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "auth")]
pub enum SmtpAuthConfig {
    /// The password authentication mechanism.
    Passwd(PasswdConfig),

    /// The OAuth 2.0 authentication mechanism.
    OAuth2(OAuth2Config),
}

impl Default for SmtpAuthConfig {
    fn default() -> Self {
        Self::Passwd(PasswdConfig::default())
    }
}

impl SmtpAuthConfig {
    /// Resets the OAuth 2.0 authentication tokens.
    pub async fn reset(&self) -> Result<()> {
        debug!("resetting smtp backend configuration");

        if let Self::OAuth2(oauth2) = self {
            oauth2.reset().await?;
        }

        Ok(())
    }

    /// Configures the OAuth 2.0 authentication tokens.
    pub async fn configure(
        &self,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        debug!("configuring smtp backend");

        if let Self::OAuth2(oauth2) = self {
            oauth2.configure(get_client_secret).await?;
        }

        Ok(())
    }

    pub fn replace_undefined_keyring_entries(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();

        match self {
            SmtpAuthConfig::Passwd(secret) => {
                secret.set_keyring_entry_if_undefined(format!("{name}-smtp-passwd"));
            }
            SmtpAuthConfig::OAuth2(config) => {
                config
                    .client_secret
                    .set_keyring_entry_if_undefined(format!("{name}-smtp-oauth2-client-secret"));
                config
                    .access_token
                    .set_keyring_entry_if_undefined(format!("{name}-smtp-oauth2-access-token"));
                config
                    .refresh_token
                    .set_keyring_entry_if_undefined(format!("{name}-smtp-oauth2-refresh-token"));
            }
        }
    }
}
