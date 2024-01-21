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
    backend::BackendContextBuilder,
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
    pub account_config: AccountConfig,

    /// The Notmuch configuration.
    pub notmuch_config: NotmuchConfig,

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
    pub account_config: AccountConfig,

    /// The Notmuch configuration.
    pub notmuch_config: NotmuchConfig,

    inner: Arc<Mutex<NotmuchContext>>,
}

impl Deref for NotmuchContextSync {
    type Target = Arc<Mutex<NotmuchContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// The Notmuch context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotmuchContextBuilder {
    /// The Notmuch configuration.
    pub config: NotmuchConfig,
}

impl NotmuchContextBuilder {
    pub fn new(config: NotmuchConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl BackendContextBuilder for NotmuchContextBuilder {
    type Context = NotmuchContextSync;

    async fn build(self, account_config: &AccountConfig) -> Result<Self::Context> {
        info!("building new notmuch context");

        let root = Maildir::from(self.config.get_maildir_path()?);

        let maildir_config = MaildirConfig {
            root_dir: root.path().to_owned(),
        };

        let mdir_ctx = MaildirContext {
            account_config: account_config.clone(),
            maildir_config,
            root,
        };

        let ctx = NotmuchContext {
            account_config: account_config.clone(),
            notmuch_config: self.config.clone(),
            mdir_ctx,
        };

        Ok(NotmuchContextSync {
            account_config: account_config.clone(),
            notmuch_config: self.config,
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
