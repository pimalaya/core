//! SMTP config module.
//!
//! This module contains the representation of the SMTP email sender
//! configuration of the user account.

use log::debug;
use mail_send::Credentials;
use std::{io, result};
use thiserror::Error;

use crate::{account, OAuth2Config, OAuth2Method, PasswdConfig};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get smtp password")]
    GetPasswdError(#[source] pimalaya_secret::Error),
    #[error("cannot get smtp password: password is empty")]
    GetPasswdEmptyError,
    #[error("cannot get smtp oauth2 access token")]
    GetOAuth2AccessTokenError(#[source] pimalaya_secret::Error),

    #[error(transparent)]
    AccountConfigError(#[from] account::config::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the internal sender config.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SmtpConfig {
    /// Represents the SMTP server host.
    pub host: String,
    /// Represents the SMTP server port.
    pub port: u16,
    /// Enables SSL.
    pub ssl: Option<bool>,
    /// Enables StartTLS.
    pub starttls: Option<bool>,
    /// Trusts any certificate.
    pub insecure: Option<bool>,
    /// Represents the SMTP server login.
    pub login: String,
    /// Represents the SMTP authentication configuration.
    pub auth: SmtpAuthConfig,
}

impl SmtpConfig {
    pub fn credentials(&self) -> Result<Credentials<String>> {
        Ok(match &self.auth {
            SmtpAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().map_err(Error::GetPasswdError)?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or_else(|| Error::GetPasswdEmptyError)?;
                Credentials::new(self.login.clone(), passwd.to_owned())
            }
            SmtpAuthConfig::OAuth2(oauth2) => match oauth2.method {
                OAuth2Method::XOAuth2 => {
                    Credentials::new_xoauth2(self.login.clone(), oauth2.access_token()?)
                }
                OAuth2Method::OAuthBearer => Credentials::new_oauth(oauth2.access_token()?),
            },
        })
    }

    pub fn ssl(&self) -> bool {
        self.ssl.unwrap_or(true)
    }

    pub fn starttls(&self) -> bool {
        self.starttls.unwrap_or_default()
    }

    pub fn insecure(&self) -> bool {
        self.insecure.unwrap_or_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmtpAuthConfig {
    Passwd(PasswdConfig),
    OAuth2(OAuth2Config),
}

impl Default for SmtpAuthConfig {
    fn default() -> Self {
        Self::Passwd(PasswdConfig::default())
    }
}

impl SmtpAuthConfig {
    pub fn reset(&self) -> Result<()> {
        debug!("resetting smtp backend configuration");

        if let Self::OAuth2(oauth2) = self {
            oauth2.reset()?;
        }

        Ok(())
    }

    pub fn configure(&self, get_client_secret: impl Fn() -> io::Result<String>) -> Result<()> {
        debug!("configuring smtp backend");

        if let Self::OAuth2(oauth2) = self {
            oauth2.configure(get_client_secret)?;
        }

        Ok(())
    }
}
