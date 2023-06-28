//! Module dedicated to account management.
//!
//! This module contains [configuration](config) related to account.
//!
//! You also have everything you need to [synchronize](sync) a remote
//! account using a local Maildir backend.

pub mod config;
pub mod sync;

pub use self::config::{
    AccountConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig, DEFAULT_DRAFTS_FOLDER,
    DEFAULT_INBOX_FOLDER, DEFAULT_PAGE_SIZE, DEFAULT_SENT_FOLDER, DEFAULT_SIGNATURE_DELIM,
};
pub use self::sync::{
    AccountSyncBuilder, AccountSyncProgress, AccountSyncProgressEvent, AccountSyncReport,
};
