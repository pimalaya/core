//! SMTP config module.
//!
//! This module contains the representation of the SMTP email sender
//! configuration of the user account.

use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use log::debug;
use std::{io, result};
use thiserror::Error;

use crate::{account, OAuth2Config, PasswdConfig};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get smtp password")]
    GetPasswdError(#[source] pimalaya_secret::Error),
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
    pub fn credentials(&self) -> Result<Credentials> {
        Ok(Credentials::new(
            self.login.clone(),
            match &self.auth {
                SmtpAuthConfig::Passwd(secret) => secret.get().map_err(Error::GetPasswdError),
                SmtpAuthConfig::OAuth2(oauth2) => Ok(oauth2.access_token()?),
            }?,
        ))
    }

    pub fn mechanism(&self) -> Mechanism {
        match self.auth {
            SmtpAuthConfig::Passwd(_) => Mechanism::Login,
            SmtpAuthConfig::OAuth2(_) => Mechanism::Xoauth2,
        }
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
