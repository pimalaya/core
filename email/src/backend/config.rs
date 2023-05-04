//! Backend config module.
//!
//! This module contains the representation of the backend
//! configuration of the user account.

use std::{fmt, result};
use thiserror::Error;

#[cfg(feature = "imap-backend")]
use crate::{imap, ImapConfig};

use crate::MaildirConfig;

#[cfg(feature = "notmuch-backend")]
use crate::NotmuchConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "imap-backend")]
    #[error("cannot configure imap backend")]
    ConfigureError(#[from] imap::config::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the backend configuration of the user account.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BackendConfig {
    None,
    Maildir(MaildirConfig),
    #[cfg(feature = "imap-backend")]
    Imap(ImapConfig),
    #[cfg(feature = "notmuch-backend")]
    Notmuch(NotmuchConfig),
}

impl BackendConfig {
    pub fn configure<N>(&self, name: N) -> Result<()>
    where
        N: fmt::Display,
    {
        match self {
            Self::Imap(config) => Ok(config.configure(name)?),
            _ => Ok(()),
        }
    }
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self::None
    }
}
