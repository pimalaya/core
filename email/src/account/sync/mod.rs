//! # Account synchronization
//!
//! Module dedicated to synchronization of folders and emails
//! belonging to an account. The main structure of this module is
//! [`AccountSyncBuilder`], which allows you to synchronize a given
//! backend with a local Maildir one, and therefore enables offline
//! support for this backend.

pub mod config;

#[doc(inline)]
pub use super::{Error, Result};
use crate::{
    backend::{context::BackendContextBuilder, BackendBuilder},
    maildir::MaildirContextBuilder,
    sync::{hash::SyncHash, SyncBuilder},
};

/// The account synchronization builder.
///
/// This builder is just a wrapper around [`SyncBuilder`], where the
/// left backend builder is a pre-defined Maildir one. The aim of this
/// builder is to provide offline support for any given backend.
pub struct AccountSyncBuilder;

impl AccountSyncBuilder {
    /// Try to create a new account synchronization builder.
    pub fn try_new<R: BackendContextBuilder + SyncHash + 'static>(
        right_builder: BackendBuilder<R>,
    ) -> Result<SyncBuilder<MaildirContextBuilder, R>> {
        let left_ctx_builder = right_builder
            .ctx_builder
            .try_to_sync_cache_builder(&right_builder.account_config)?;
        let left_builder =
            BackendBuilder::new(right_builder.account_config.clone(), left_ctx_builder);
        let sync_builder = SyncBuilder::new(left_builder, right_builder);

        Ok(sync_builder)
    }
}
