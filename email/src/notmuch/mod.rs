pub mod config;

use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use notmuch::{Database, DatabaseMode};
use shellexpand_utils::shellexpand_path;
use std::{ops::Deref, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    envelope::{
        get::{notmuch::GetNotmuchEnvelope, GetEnvelope},
        list::{notmuch::ListNotmuchEnvelopes, ListEnvelopes},
    },
    flag::{
        add::{notmuch::AddNotmuchFlags, AddFlags},
        remove::{notmuch::RemoveNotmuchFlags, RemoveFlags},
        set::{notmuch::SetNotmuchFlags, SetFlags},
    },
    folder::{
        add::{notmuch::AddNotmuchFolder, AddFolder},
        list::{notmuch::ListNotmuchFolders, ListFolders},
    },
    maildir::{config::MaildirConfig, MaildirContext},
    message::{
        add::{notmuch::AddNotmuchMessage, AddMessage},
        copy::{notmuch::CopyNotmuchMessages, CopyMessages},
        delete::{notmuch::DeleteNotmuchMessages, DeleteMessages},
        get::{notmuch::GetNotmuchMessages, GetMessages},
        peek::{notmuch::PeekNotmuchMessages, PeekMessages},
        r#move::{notmuch::MoveNotmuchMessages, MoveMessages},
    },
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

    fn check_up(&self) -> Option<BackendFeature<Self::Context, dyn CheckUp>> {
        Some(Arc::new(CheckUpNotmuch::some_new_boxed))
    }

    fn add_folder(&self) -> Option<BackendFeature<Self::Context, dyn AddFolder>> {
        Some(Arc::new(AddNotmuchFolder::some_new_boxed))
    }

    fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
        Some(Arc::new(ListNotmuchFolders::some_new_boxed))
    }

    // TODO
    // fn expunge_folder(&self) -> Option<BackendFeature<Self::Context, dyn ExpungeFolder>> {
    //     Some(Arc::new(ExpungeNotmuchFolder::some_new_boxed))
    // }

    // TODO
    // fn purge_folder(&self) -> Option<BackendFeature<Self::Context, dyn PurgeFolder>> {
    //     Some(Arc::new(PurgeNotmuchFolder::some_new_boxed))
    // }

    // TODO
    // fn delete_folder(&self) -> Option<BackendFeature<Self::Context, dyn DeleteFolder>> {
    //     Some(Arc::new(DeleteNotmuchFolder::some_new_boxed))
    // }

    fn get_envelope(&self) -> Option<BackendFeature<Self::Context, dyn GetEnvelope>> {
        Some(Arc::new(GetNotmuchEnvelope::some_new_boxed))
    }

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        Some(Arc::new(ListNotmuchEnvelopes::some_new_boxed))
    }

    // TODO
    // fn watch_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn WatchEnvelopes>> {
    //     Some(Arc::new(WatchNotmuchEnvelopes::some_new_boxed))
    // }

    fn add_flags(&self) -> Option<BackendFeature<Self::Context, dyn AddFlags>> {
        Some(Arc::new(AddNotmuchFlags::some_new_boxed))
    }

    fn set_flags(&self) -> Option<BackendFeature<Self::Context, dyn SetFlags>> {
        Some(Arc::new(SetNotmuchFlags::some_new_boxed))
    }

    fn remove_flags(&self) -> Option<BackendFeature<Self::Context, dyn RemoveFlags>> {
        Some(Arc::new(RemoveNotmuchFlags::some_new_boxed))
    }

    fn add_message(&self) -> Option<BackendFeature<Self::Context, dyn AddMessage>> {
        Some(Arc::new(AddNotmuchMessage::some_new_boxed))
    }

    fn peek_messages(&self) -> Option<BackendFeature<Self::Context, dyn PeekMessages>> {
        Some(Arc::new(PeekNotmuchMessages::some_new_boxed))
    }

    fn get_messages(&self) -> Option<BackendFeature<Self::Context, dyn GetMessages>> {
        Some(Arc::new(GetNotmuchMessages::some_new_boxed))
    }

    fn copy_messages(&self) -> Option<BackendFeature<Self::Context, dyn CopyMessages>> {
        Some(Arc::new(CopyNotmuchMessages::some_new_boxed))
    }

    fn move_messages(&self) -> Option<BackendFeature<Self::Context, dyn MoveMessages>> {
        Some(Arc::new(MoveNotmuchMessages::some_new_boxed))
    }

    fn delete_messages(&self) -> Option<BackendFeature<Self::Context, dyn DeleteMessages>> {
        Some(Arc::new(DeleteNotmuchMessages::some_new_boxed))
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

#[derive(Clone)]
pub struct CheckUpNotmuch {
    pub ctx: NotmuchContextSync,
}

impl CheckUpNotmuch {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn CheckUp> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn CheckUp>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CheckUp for CheckUpNotmuch {
    async fn check_up(&self) -> Result<()> {
        let ctx = self.ctx.lock().await;

        let db = ctx.open_db()?;
        db.create_query("*")?.count_messages()?;
        db.close()?;

        Ok(())
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
