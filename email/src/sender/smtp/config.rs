//! SMTP config module.
//!
//! This module contains the representation of the SMTP email sender
//! configuration of the user account.

use std::result;

use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use thiserror::Error;

use crate::process;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get smtp password")]
    GetPasswdError(#[source] process::Error),
    #[error("cannot get smtp password: password is empty")]
    GetPasswdEmptyError,
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the internal sender config.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
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
    /// Represents the SMTP password command.
    pub passwd_cmd: String,
}

impl SmtpConfig {
    /// Builds the internal SMTP sender credentials.
    pub fn credentials(&self) -> Result<SmtpCredentials> {
        let passwd = process::run(&self.passwd_cmd, &[]).map_err(Error::GetPasswdError)?;
        let passwd = String::from_utf8_lossy(&passwd).to_string();
        let passwd = passwd
            .lines()
            .next()
            .ok_or_else(|| Error::GetPasswdEmptyError)?;
        Ok(SmtpCredentials::new(
            self.login.to_owned(),
            passwd.to_owned(),
        ))
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
