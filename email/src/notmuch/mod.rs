pub mod config;

use async_trait::async_trait;
use log::info;
use notmuch::{Database, DatabaseMode};
use shellexpand_utils::shellexpand_path;
use std::{ops::Deref, path::PathBuf, sync::Arc};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{account::config::AccountConfig, backend::BackendContextBuilder, Result};

use self::config::NotmuchConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open notmuch database at {1}")]
    OpenDatabaseError(#[source] notmuch::Error, PathBuf),
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
}

impl NotmuchContext {
    pub fn open_db(&self) -> Result<Database> {
        let path = shellexpand_path(&self.notmuch_config.db_path);

        let db =
            Database::open_with_config(Some(&path), DatabaseMode::ReadWrite, None::<PathBuf>, None)
                .map_err(|err| Error::OpenDatabaseError(err, path))?;

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

impl From<NotmuchContext> for NotmuchContextSync {
    fn from(ctx: NotmuchContext) -> Self {
        Self {
            account_config: ctx.account_config.clone(),
            notmuch_config: ctx.notmuch_config.clone(),
            inner: Arc::new(Mutex::new(ctx)),
        }
    }
}

/// The Notmuch context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NotmuchContextBuilder {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The Notmuch configuration.
    pub notmuch_config: NotmuchConfig,
}

impl NotmuchContextBuilder {
    pub fn new(account_config: AccountConfig, notmuch_config: NotmuchConfig) -> Self {
        Self {
            account_config,
            notmuch_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for NotmuchContextBuilder {
    type Context = NotmuchContextSync;

    async fn build(self) -> Result<Self::Context> {
        info!("building new notmuch context");

        let context = NotmuchContext {
            account_config: self.account_config,
            notmuch_config: self.notmuch_config,
        };

        Ok(context.into())
    }
}

// const EXTRACT_FOLDER_FROM_QUERY: Lazy<Regex> =
//     Lazy::new(|| Regex::new("folder:\"?([^\"]*)\"?").unwrap());

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

//     async fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String> {
//         info!(
//             "adding notmuch email with flags {flags}",
//             flags = flags.to_string()
//         );

//         let db = self.open_db()?;

//         let folder_alias = self.account_config.find_folder_alias(folder)?;
//         let folder = match folder_alias {
//             Some(ref alias) => EXTRACT_FOLDER_FROM_QUERY
//                 .captures(alias)
//                 .map(|m| m[1].to_owned())
//                 .unwrap_or_else(|| folder.to_owned()),
//             None => folder.to_owned(),
//         };
//         let path = self.path().join(folder);
//         let mdir = Maildir::from(
//             path.canonicalize()
//                 .map_err(|err| Error::CanonicalizePathError(err, path.clone()))?,
//         );
//         let mdir_internal_id = mdir
//             .store_cur_with_flags(email, &flags.to_mdir_string())
//             .map_err(|err| Error::StoreWithFlagsError(err, mdir.path().to_owned()))?;
//         trace!("added email internal maildir id: {mdir_internal_id}");

//         let entry = mdir
//             .find(&mdir_internal_id)
//             .ok_or(Error::FindMaildirEmailById)?;
//         let path = entry
//             .path()
//             .canonicalize()
//             .map_err(|err| Error::CanonicalizePathError(err, entry.path().clone()))?;
//         trace!("path: {path:?}");

//         let email = db.index_file(&path, None).map_err(Error::IndexFileError)?;

//         Self::close_db(db)?;
//         Ok(email.id().to_string())
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

// /// The Notmuch backend builder.
// ///
// /// Simple builder that helps to build a Notmuch backend.
// pub struct NotmuchBackendBuilder {
//     account_config: AccountConfig,
//     notmuch_config: NotmuchConfig,
// }

// impl NotmuchBackendBuilder {
//     /// Creates a new builder from configurations.
//     pub fn new(account_config: AccountConfig, notmuch_config: NotmuchConfig) -> Self {
//         Self {
//             account_config,
//             notmuch_config,
//         }
//     }

//     /// Builds the Notmuch backend.
//     pub fn build(&self) -> Result<NotmuchBackend> {
//         Ok(NotmuchBackend::new(
//             self.account_config.clone(),
//             self.notmuch_config.clone(),
//         )?)
//     }
// }
