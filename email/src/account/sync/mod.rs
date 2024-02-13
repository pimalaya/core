//! # Account synchronization
//!
//! Module dedicated to synchronization of folders and emails
//! belonging to an account. The main structure of this module is
//! [`AccountSyncBuilder`], which allows you to synchronize a given
//! backend with a local Maildir one, and therefore enables offline
//! support for this backend.

pub mod config;

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    backend::{BackendBuilder, BackendContextBuilder},
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    sync::SyncBuilder,
    Result,
};

/// The account synchronization builder.
///
/// This builder is just a wrapper around [`SyncBuilder`], where the
/// left backend builder is a pre-defined Maildir one. The aim of this
/// builder is to provide offline support for any given backend.
pub struct AccountSyncBuilder<R: BackendContextBuilder>(SyncBuilder<MaildirContextBuilder, R>);

impl<R: BackendContextBuilder + 'static> AccountSyncBuilder<R> {
    /// Create a new account synchronization builder.
    pub async fn new(right_builder: BackendBuilder<R>) -> Result<Self> {
        let account_config = right_builder.account_config.clone();
        let sync_dir = account_config.get_sync_dir()?;
        let mdir_config = Arc::new(MaildirConfig { root_dir: sync_dir });
        let ctx_builder = MaildirContextBuilder::new(mdir_config);
        let left_builder = BackendBuilder::new(account_config, ctx_builder);
        let sync_builder = SyncBuilder::new(left_builder, right_builder);

        Ok(Self(sync_builder))
    }
}

impl<R: BackendContextBuilder> Deref for AccountSyncBuilder<R> {
    type Target = SyncBuilder<MaildirContextBuilder, R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<R: BackendContextBuilder> DerefMut for AccountSyncBuilder<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
