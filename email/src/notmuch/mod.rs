pub mod config;

use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use notmuch::{Database, DatabaseMode};
use shellexpand_utils::shellexpand_path;
use std::{ops::Deref, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

#[cfg(feature = "envelope-get")]
use crate::envelope::get::{notmuch::GetNotmuchEnvelope, GetEnvelope};
#[cfg(feature = "envelope-list")]
use crate::envelope::list::{notmuch::ListNotmuchEnvelopes, ListEnvelopes};
// TODO
// #[cfg(feature = "envelope-watch")]
// use crate::envelope::watch::{notmuch::WatchNotmuchEnvelopes, WatchEnvelopes};
#[cfg(feature = "flag-add")]
use crate::flag::add::{notmuch::AddNotmuchFlags, AddFlags};
#[cfg(feature = "flag-remove")]
use crate::flag::remove::{notmuch::RemoveNotmuchFlags, RemoveFlags};
#[cfg(feature = "flag-set")]
use crate::flag::set::{notmuch::SetNotmuchFlags, SetFlags};
#[cfg(feature = "folder-add")]
use crate::folder::add::{notmuch::AddNotmuchFolder, AddFolder};
// TODO
// #[cfg(feature = "folder-delete")]
// use crate::folder::delete::{notmuch::DeleteNotmuchFolder, DeleteFolder};
// TODO
// #[cfg(feature = "folder-expunge")]
// use crate::folder::expunge::{notmuch::ExpungeNotmuchFolder, ExpungeFolder};
#[cfg(feature = "folder-list")]
use crate::folder::list::{notmuch::ListNotmuchFolders, ListFolders};
// TODO
// #[cfg(feature = "folder-purge")]
// use crate::folder::purge::{notmuch::PurgeNotmuchFolder, PurgeFolder};
#[cfg(feature = "message-add")]
use crate::message::add::{notmuch::AddNotmuchMessage, AddMessage};
#[cfg(feature = "message-copy")]
use crate::message::copy::{notmuch::CopyNotmuchMessages, CopyMessages};
#[cfg(feature = "message-delete")]
use crate::message::delete::{notmuch::DeleteNotmuchMessages, DeleteMessages};
#[cfg(feature = "message-get")]
use crate::message::get::{notmuch::GetNotmuchMessages, GetMessages};
#[cfg(feature = "message-peek")]
use crate::message::peek::{notmuch::PeekNotmuchMessages, PeekMessages};
#[cfg(feature = "message-move")]
use crate::message::r#move::{notmuch::MoveNotmuchMessages, MoveMessages};
use crate::{
    account::config::AccountConfig,
    backend::{BackendContext, BackendContextBuilder, BackendFeatureBuilder},
    maildir::{config::MaildirConfig, MaildirContext},
    Result,
};

use self::config::NotmuchConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open notmuch database")]
    OpenDatabaseError(#[source] notmuch::Error),
}

/// The Notmuch backend context.
///
/// The Notmuch database internally uses `Rc` which prevents it to be
/// `Send` and therefore to be attached to this backend context. A new
/// database needs to be opened and closed for every action.
///
/// See <https://github.com/vhdirk/notmuch-rs/issues/48>.
pub struct NotmuchContext {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The Notmuch configuration.
    pub notmuch_config: Arc<NotmuchConfig>,

    /// The Maildir context associated to the Notmuch database.
    pub mdir_ctx: MaildirContext,
}

impl NotmuchContext {
    pub fn open_db(&self) -> Result<Database> {
        let db_path = self
            .notmuch_config
            .database_path
            .as_ref()
            .map(shellexpand_path);
        let db_mode = DatabaseMode::ReadWrite;
        let config_path = self.notmuch_config.find_config_path();
        let profile = self.notmuch_config.find_profile();

        let db = Database::open_with_config(db_path, db_mode, config_path, profile)
            .map_err(Error::OpenDatabaseError)?;

        Ok(db)
    }
}

/// The sync version of the Notmuch backend context.
///
/// For now, the Notmuch sync backend context is not so useful, it is
/// the same as the Notmuch unsync backend context. The struct is
/// there in case one day the database becomes thread-safe.
#[derive(Clone)]
pub struct NotmuchContextSync {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The Notmuch configuration.
    pub notmuch_config: Arc<NotmuchConfig>,

    inner: Arc<Mutex<NotmuchContext>>,
}

impl Deref for NotmuchContextSync {
    type Target = Arc<Mutex<NotmuchContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BackendContext for NotmuchContextSync {}

/// The Notmuch context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotmuchContextBuilder {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The Notmuch configuration.
    pub notmuch_config: Arc<NotmuchConfig>,
}

impl NotmuchContextBuilder {
    pub fn new(account_config: Arc<AccountConfig>, notmuch_config: Arc<NotmuchConfig>) -> Self {
        Self {
            account_config,
            notmuch_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for NotmuchContextBuilder {
    type Context = NotmuchContextSync;

    #[cfg(feature = "folder-add")]
    fn add_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFolder> {
        BackendFeatureBuilder::new(AddNotmuchFolder::some_new_boxed)
    }

    #[cfg(feature = "folder-list")]
    fn list_folders(&self) -> BackendFeatureBuilder<Self::Context, dyn ListFolders> {
        BackendFeatureBuilder::new(ListNotmuchFolders::some_new_boxed)
    }

    // TODO
    // #[cfg(feature = "folder-expunge")]
    // fn expunge_folder(
    //     &self,
    // ) -> BackendFeatureBuilder<Self::Context, dyn ExpungeFolder> {
    //     BackendFeatureBuilder::new(ExpungeNotmuchFolder::some_new_boxed)
    // }

    // TODO
    // #[cfg(feature = "folder-purge")]
    // fn purge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn PurgeFolder> {
    //     BackendFeatureBuilder::new(PurgeNotmuchFolder::some_new_boxed)
    // }

    // TODO
    // #[cfg(feature = "folder-delete")]
    // fn delete_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteFolder> {
    //     BackendFeatureBuilder::new(DeleteNotmuchFolder::some_new_boxed)
    // }

    #[cfg(feature = "envelope-list")]
    fn list_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn ListEnvelopes> {
        BackendFeatureBuilder::new(ListNotmuchEnvelopes::some_new_boxed)
    }

    // TODO
    // #[cfg(feature = "envelope-watch")]
    // fn watch_envelopes(
    //     &self,
    // ) -> BackendFeatureBuilder<Self::Context, dyn WatchEnvelopes> {
    //     BackendFeatureBuilder::new(WatchNotmuchEnvelopes::some_new_boxed)
    // }

    #[cfg(feature = "envelope-get")]
    fn get_envelope(&self) -> BackendFeatureBuilder<Self::Context, dyn GetEnvelope> {
        BackendFeatureBuilder::new(GetNotmuchEnvelope::some_new_boxed)
    }

    #[cfg(feature = "flag-add")]
    fn add_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFlags> {
        BackendFeatureBuilder::new(AddNotmuchFlags::some_new_boxed)
    }

    #[cfg(feature = "flag-set")]
    fn set_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn SetFlags> {
        BackendFeatureBuilder::new(SetNotmuchFlags::some_new_boxed)
    }

    #[cfg(feature = "flag-remove")]
    fn remove_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn RemoveFlags> {
        BackendFeatureBuilder::new(RemoveNotmuchFlags::some_new_boxed)
    }

    #[cfg(feature = "message-add")]
    fn add_message(&self) -> BackendFeatureBuilder<Self::Context, dyn AddMessage> {
        BackendFeatureBuilder::new(AddNotmuchMessage::some_new_boxed)
    }

    #[cfg(feature = "message-peek")]
    fn peek_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn PeekMessages> {
        BackendFeatureBuilder::new(PeekNotmuchMessages::some_new_boxed)
    }

    #[cfg(feature = "message-get")]
    fn get_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn GetMessages> {
        BackendFeatureBuilder::new(GetNotmuchMessages::some_new_boxed)
    }

    #[cfg(feature = "message-copy")]
    fn copy_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn CopyMessages> {
        BackendFeatureBuilder::new(CopyNotmuchMessages::some_new_boxed)
    }

    #[cfg(feature = "message-move")]
    fn move_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn MoveMessages> {
        BackendFeatureBuilder::new(MoveNotmuchMessages::some_new_boxed)
    }

    #[cfg(feature = "message-delete")]
    fn delete_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteMessages> {
        BackendFeatureBuilder::new(DeleteNotmuchMessages::some_new_boxed)
    }

    async fn build(self) -> Result<Self::Context> {
        info!("building new notmuch context");

        let root = Maildir::from(self.notmuch_config.get_maildir_path()?);

        let maildir_config = Arc::new(MaildirConfig {
            root_dir: root.path().to_owned(),
        });

        let mdir_ctx = MaildirContext {
            account_config: self.account_config.clone(),
            maildir_config,
            root,
        };

        let ctx = NotmuchContext {
            account_config: self.account_config.clone(),
            notmuch_config: self.notmuch_config.clone(),
            mdir_ctx,
        };

        Ok(NotmuchContextSync {
            account_config: self.account_config,
            notmuch_config: self.notmuch_config,
            inner: Arc::new(Mutex::new(ctx)),
        })
    }
}

// #[async_trait]
// impl Backend for NotmuchBackend {
//     async fn search_envelopes(
//         &mut self,
//         folder: &str,
//         query: &str,
//         _sort: &str,
//         page_size: usize,
//         page: usize,
//     ) -> Result<Envelopes> {
//         info!("searching notmuch envelopes from folder {folder}");

//         let folder_query = self
//             .account_config
//             .find_folder_alias(folder.as_ref())?
//             .unwrap_or_else(|| format!("folder:{folder:?}"));
//         let query = if query.is_empty() {
//             folder_query
//         } else {
//             folder_query + " and " + query.as_ref()
//         };
//         debug!("notmuch query: {query}");

//         let envelopes = self._search_envelopes(&query, page_size, page)?;

//         Ok(envelopes)
//     }

//     async fn delete_emails(&mut self, _folder: &str, internal_ids: Vec<&str>) -> Result<()> {
//         info!(
//             "deleting notmuch emails by internal ids {ids}",
//             ids = internal_ids.join(", ")
//         );

//         let db = self.open_db()?;

//         internal_ids.iter().try_for_each(|internal_id| {
//             let path = db
//                 .find_message(&internal_id)
//                 .map_err(Error::FindEmailError)?
//                 .ok_or_else(|| Error::FindMsgEmptyError)?
//                 .filename()
//                 .to_owned();
//             db.remove_message(path).map_err(Error::DelMsgError)
//         })?;

//         Self::close_db(db)?;
//         Ok(())
//     }
// }
