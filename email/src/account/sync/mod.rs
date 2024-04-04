//! # Account synchronization
//!
//! Module dedicated to synchronization of folders and emails
//! belonging to an account. The main structure of this module is
//! [`AccountSyncBuilder`], which allows you to synchronize a given
//! backend with a local Maildir one, and therefore enables offline
//! support for this backend.

pub mod config;

use std::{
    hash::{DefaultHasher, Hasher},
    sync::Arc,
};

use dirs::data_dir;
use log::debug;
use shellexpand_utils::try_shellexpand_path;

use crate::{
    account::error::Error,
    backend::{context::BackendContextBuilder, BackendBuilder},
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    sync::{hash::SyncHash, SyncBuilder},
};

use super::config::AccountConfig;

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
    ) -> Result<SyncBuilder<MaildirContextBuilder, R>, Error> {
        let mut right_hasher = DefaultHasher::new();
        right_builder.sync_hash(&mut right_hasher);
        let right_hash = format!("{:x}", right_hasher.finish());

        let right_sync_dir = right_builder
            .account_config
            .sync
            .as_ref()
            .and_then(|c| c.dir.as_ref());

        let left_account_config = Arc::new(AccountConfig {
            name: right_builder.account_config.name.clone(),
            email: right_builder.account_config.email.clone(),
            display_name: right_builder.account_config.display_name.clone(),
            signature: right_builder.account_config.signature.clone(),
            signature_delim: right_builder.account_config.signature_delim.clone(),
            downloads_dir: right_builder.account_config.downloads_dir.clone(),
            folder: right_builder.account_config.folder.clone(),
            envelope: right_builder.account_config.envelope.clone(),
            flag: right_builder.account_config.flag.clone(),
            message: right_builder.account_config.message.clone(),
            template: right_builder.account_config.template.clone(),
            sync: None,
            #[cfg(feature = "pgp")]
            pgp: right_builder.account_config.pgp.clone(),
        });

        let left_config = Arc::new(MaildirConfig {
            root_dir: match right_sync_dir {
                Some(dir) => {
                    let sync_dir = try_shellexpand_path(dir)
                        .map_err(|err| Error::GetSyncDirInvalidError(err, dir.clone()))?;
                    debug!("using custom sync dir {sync_dir:?}");
                    Result::<_, Error>::Ok(sync_dir)
                }
                None => {
                    let sync_dir = data_dir()
                        .ok_or(Error::GetXdgDataDirSyncError)?
                        .join("pimalaya")
                        .join("email")
                        .join("sync")
                        .join(&right_hash);
                    debug!("using default sync dir {sync_dir:?}");
                    Result::<_, Error>::Ok(sync_dir)
                }
            }?,
        });

        let left_ctx = MaildirContextBuilder::new(left_account_config.clone(), left_config);
        let left_builder = BackendBuilder::new(left_account_config, left_ctx);

        Ok(SyncBuilder::new(left_builder, right_builder))
    }
}
