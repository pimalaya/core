//! # Backend context
//!
//! The [`BackendContext`] is usually used for storing clients or
//! sessions (structures than cannot be cloned or sync). The
//! [`BackendContextBuilder`] gives instructions on how to build such
//! context. It is used by the backend builder.

use async_trait::async_trait;
use paste::paste;

use crate::{
    envelope::{get::GetEnvelope, list::ListEnvelopes, watch::WatchEnvelopes},
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder,
    },
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        peek::PeekMessages, r#move::MoveMessages, remove::RemoveMessages, send::SendMessage,
    },
    AnyResult,
};

use super::feature::{BackendFeature, CheckUp};

/// The backend context.
///
/// This is just a marker for other backend traits. Every backend
/// context needs to implement this trait manually or to derive
/// [`crate::backend_v2::macros::BackendContextV2`].
pub trait BackendContext: Send + Sync {}

/// Macro for defining [`BackendContextBuilder`] features.
macro_rules! feature {
    ($feat:ty) => {
        paste! {
            /// Define the given backend feature.
            fn [<$feat:snake>](&self) -> Option<BackendFeature<Self::Context, dyn $feat>> {
                None
            }
        }
    };
}

/// The backend context builder.
///
/// This trait defines how a context should be built. It also defines
/// default backend features implemented by the context itself.
#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
    /// The type of the context being built by this builder.
    type Context: BackendContext;

    fn check_configuration(&self) -> AnyResult<()> {
        Ok(())
    }

    async fn configure(&mut self) -> AnyResult<()> {
        Ok(())
    }

    feature!(CheckUp);

    feature!(AddFolder);
    feature!(ListFolders);
    feature!(ExpungeFolder);
    feature!(PurgeFolder);
    feature!(DeleteFolder);
    feature!(GetEnvelope);
    feature!(ListEnvelopes);
    feature!(WatchEnvelopes);
    feature!(AddFlags);
    feature!(SetFlags);
    feature!(RemoveFlags);
    feature!(AddMessage);
    feature!(SendMessage);
    feature!(PeekMessages);
    feature!(GetMessages);
    feature!(CopyMessages);
    feature!(MoveMessages);
    feature!(DeleteMessages);
    feature!(RemoveMessages);

    /// Build the final context used by the backend.
    async fn build(self) -> AnyResult<Self::Context>;

    #[cfg(feature = "account-sync")]
    fn try_to_sync_cache_builder(
        &self,
        account_config: &crate::account::config::AccountConfig,
    ) -> std::result::Result<crate::maildir::MaildirContextBuilder, crate::account::Error>
    where
        Self: crate::sync::hash::SyncHash,
    {
        use crate::debug;
        use dirs::data_dir;
        use shellexpand_utils::try_shellexpand_path;
        use std::{
            hash::{DefaultHasher, Hasher},
            sync::Arc,
        };

        use crate::{
            account::{config::AccountConfig, Error},
            maildir::{config::MaildirConfig, MaildirContextBuilder},
        };

        let mut hasher = DefaultHasher::new();
        self.sync_hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());

        let account_config = Arc::new(AccountConfig {
            name: account_config.name.clone(),
            email: account_config.email.clone(),
            display_name: account_config.display_name.clone(),
            signature: account_config.signature.clone(),
            signature_delim: account_config.signature_delim.clone(),
            downloads_dir: account_config.downloads_dir.clone(),
            folder: account_config.folder.clone(),
            envelope: account_config.envelope.clone(),
            flag: account_config.flag.clone(),
            message: account_config.message.clone(),
            template: account_config.template.clone(),
            sync: None,
            #[cfg(feature = "pgp")]
            pgp: account_config.pgp.clone(),
        });

        let sync_dir = account_config.sync.as_ref().and_then(|c| c.dir.as_ref());
        let root_dir = match sync_dir {
            Some(dir) => {
                let sync_dir = try_shellexpand_path(dir)
                    .map_err(|err| Error::GetSyncDirInvalidError(err, dir.clone()))?;
                debug!("using custom sync dir {sync_dir:?}");
                sync_dir
            }
            None => {
                let sync_dir = data_dir()
                    .ok_or(Error::GetXdgDataDirSyncError)?
                    .join("pimalaya")
                    .join("email")
                    .join("sync")
                    .join(&hash);
                debug!("using default sync dir {sync_dir:?}");
                sync_dir
            }
        };
        let config = Arc::new(MaildirConfig { root_dir });
        let ctx = MaildirContextBuilder::new(account_config.clone(), config);

        Ok(ctx)
    }
}
