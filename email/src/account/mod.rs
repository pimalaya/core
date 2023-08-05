//! Module dedicated to account management.
//!
//! This module contains everything related to account configuration,
//! plus everything you need to synchronize a remote account using a
//! local Maildir backend. It also contains common code related to
//! PGP.

pub mod config;
pub mod sync;

#[cfg(feature = "gpg")]
#[doc(inline)]
pub use self::config::GpgConfig;
#[doc(inline)]
pub use self::config::{
    AccountConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig, PgpConfig,
    PgpNativeConfig, PgpNativeSecretKey, SignedSecretKey, DEFAULT_DRAFTS_FOLDER,
    DEFAULT_INBOX_FOLDER, DEFAULT_PAGE_SIZE, DEFAULT_SENT_FOLDER, DEFAULT_SIGNATURE_DELIM,
    DEFAULT_TRASH_FOLDER,
};
