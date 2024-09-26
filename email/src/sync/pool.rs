use std::{collections::BTreeSet, sync::Arc};

#[doc(inline)]
pub use super::{Error, Result};
use super::{SyncDestination, SyncEventHandler};
use crate::{
    backend::{
        context::{BackendContext, BackendContextBuilder},
        Backend, BackendBuilder,
    },
    email::sync::hunk::EmailSyncHunk,
    envelope::sync::config::EnvelopeSyncFilters,
    flag::sync::config::FlagSyncPermissions,
    folder::sync::{
        config::{FolderSyncPermissions, FolderSyncStrategy},
        hunk::FolderSyncHunk,
        patch::FolderSyncPatches,
    },
    maildir::{MaildirContextBuilder, MaildirContextSync},
    message::sync::config::MessageSyncPermissions,
    AnyResult,
};

#[derive(Clone, Default)]
pub struct SyncPoolConfig {
    pub left_folder_permissions: Option<FolderSyncPermissions>,
    pub left_flag_permissions: Option<FlagSyncPermissions>,
    pub left_message_permissions: Option<MessageSyncPermissions>,
    pub right_folder_permissions: Option<FolderSyncPermissions>,
    pub right_flag_permissions: Option<FlagSyncPermissions>,
    pub right_message_permissions: Option<MessageSyncPermissions>,
    pub pool_size: Option<usize>,
    pub folder_filters: Option<FolderSyncStrategy>,
    pub envelope_filters: Option<EnvelopeSyncFilters>,
    pub handler: Option<Arc<SyncEventHandler>>,
    pub dry_run: Option<bool>,
}

#[derive(Clone)]
pub struct SyncPoolContextBuilder<L, R>
where
    L: BackendContextBuilder,
    R: BackendContextBuilder,
{
    config: SyncPoolConfig,
    left_cache_builder: BackendBuilder<MaildirContextBuilder>,
    left_builder: BackendBuilder<L>,
    right_cache_builder: BackendBuilder<MaildirContextBuilder>,
    right_builder: BackendBuilder<R>,
}

impl<L, R> SyncPoolContextBuilder<L, R>
where
    L: BackendContextBuilder,
    R: BackendContextBuilder,
{
    pub fn new(
        config: SyncPoolConfig,
        left_cache_builder: BackendBuilder<MaildirContextBuilder>,
        left_builder: BackendBuilder<L>,
        right_cache_builder: BackendBuilder<MaildirContextBuilder>,
        right_builder: BackendBuilder<R>,
    ) -> Self {
        Self {
            config,
            left_cache_builder,
            left_builder,
            right_cache_builder,
            right_builder,
        }
    }

    pub async fn build(self) -> AnyResult<SyncPoolContext<L::Context, R::Context>> {
        let left_folder_permissions = self
            .config
            .left_folder_permissions
            .clone()
            .or_else(|| {
                self.left_builder
                    .account_config
                    .folder
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.permissions.clone())
            })
            .unwrap_or_default();

        let left_flag_permissions = self
            .config
            .left_flag_permissions
            .clone()
            .or_else(|| {
                self.left_builder
                    .account_config
                    .flag
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.permissions.clone())
            })
            .unwrap_or_default();

        let left_message_permissions = self
            .config
            .left_message_permissions
            .clone()
            .or_else(|| {
                self.left_builder
                    .account_config
                    .message
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.permissions.clone())
            })
            .unwrap_or_default();

        let right_folder_permissions = self
            .config
            .right_folder_permissions
            .clone()
            .or_else(|| {
                self.right_builder
                    .account_config
                    .folder
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.permissions.clone())
            })
            .unwrap_or_default();

        let right_flag_permissions = self
            .config
            .right_flag_permissions
            .clone()
            .or_else(|| {
                self.right_builder
                    .account_config
                    .flag
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.permissions.clone())
            })
            .unwrap_or_default();

        let right_message_permissions = self
            .config
            .right_message_permissions
            .clone()
            .or_else(|| {
                self.right_builder
                    .account_config
                    .message
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.permissions.clone())
            })
            .unwrap_or_default();

        let folder_filters = self
            .config
            .folder_filters
            .clone()
            .or_else(|| {
                self.right_builder
                    .account_config
                    .folder
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.filter.clone())
            })
            .unwrap_or_default();

        let envelope_filters = self
            .config
            .envelope_filters
            .clone()
            .or_else(|| {
                self.right_builder
                    .account_config
                    .envelope
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.filter.clone())
            })
            .unwrap_or_default();

        let (left_cache, left, right_cache, right) = tokio::try_join!(
            self.left_cache_builder.build(),
            self.left_builder.build(),
            self.right_cache_builder.build(),
            self.right_builder.build(),
        )?;

        Ok(SyncPoolContext {
            left_cache,
            left,
            left_folder_permissions,
            left_flag_permissions,
            left_message_permissions,
            right_cache,
            right,
            right_folder_permissions,
            right_flag_permissions,
            right_message_permissions,
            folder_filters,
            envelope_filters,
            handler: self.config.handler,
            dry_run: self.config.dry_run.unwrap_or_default(),
        })
    }
}

pub struct SyncPoolContext<L: BackendContext, R: BackendContext> {
    pub left_cache: Backend<MaildirContextSync>,
    pub left: Backend<L>,
    pub right_cache: Backend<MaildirContextSync>,
    pub right: Backend<R>,
    pub left_folder_permissions: FolderSyncPermissions,
    pub left_flag_permissions: FlagSyncPermissions,
    pub left_message_permissions: MessageSyncPermissions,
    pub right_folder_permissions: FolderSyncPermissions,
    pub right_flag_permissions: FlagSyncPermissions,
    pub right_message_permissions: MessageSyncPermissions,
    pub folder_filters: FolderSyncStrategy,
    pub envelope_filters: EnvelopeSyncFilters,
    pub handler: Option<Arc<SyncEventHandler>>,
    pub dry_run: bool,
}

impl<L: BackendContext, R: BackendContext> SyncPoolContext<L, R> {
    pub fn apply_folder_permissions(&self, patch: &mut FolderSyncPatches) {
        use FolderSyncHunk::*;
        use SyncDestination::*;

        for (_, patch) in patch.iter_mut() {
            patch.retain(|hunk| match hunk {
                Create(_, Left) | Cache(_, Left) => self.left_folder_permissions.create,
                Create(_, Right) | Cache(_, Right) => self.right_folder_permissions.create,
                Delete(_, Left) | Uncache(_, Left) => self.left_folder_permissions.delete,
                Delete(_, Right) | Uncache(_, Right) => self.right_folder_permissions.delete,
            });
        }
    }

    pub fn apply_flag_and_message_permissions(&self, patch: &mut BTreeSet<EmailSyncHunk>) {
        use EmailSyncHunk::*;
        use SyncDestination::*;

        patch.retain(|hunk| match hunk {
            GetThenCache(_, _, Left) => self.left_message_permissions.create,
            GetThenCache(_, _, Right) => self.right_message_permissions.create,
            CopyThenCache(_, _, _, Left, _) => self.left_message_permissions.create,
            CopyThenCache(_, _, _, Right, _) => self.right_message_permissions.create,
            UpdateCachedFlags(_, _, Left) => self.left_flag_permissions.update,
            UpdateCachedFlags(_, _, Right) => self.right_flag_permissions.update,
            UpdateFlags(_, _, Left) => self.left_flag_permissions.update,
            UpdateFlags(_, _, Right) => self.right_flag_permissions.update,
            Uncache(_, _, Left) | Delete(_, _, Left) => self.left_message_permissions.delete,
            Uncache(_, _, Right) | Delete(_, _, Right) => self.right_message_permissions.delete,
        });
    }
}
