pub mod config;
mod error;

use std::{ops::Deref, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use maildirs::{Maildir, Maildirs};
use shellexpand_utils::{shellexpand_path, try_shellexpand_path};
use tokio::sync::Mutex;
use tracing::info;

use self::config::MaildirConfig;
#[doc(inline)]
pub use self::error::{Error, Result};
#[cfg(feature = "thread")]
use crate::envelope::thread::{maildir::ThreadMaildirEnvelopes, ThreadEnvelopes};
#[cfg(feature = "watch")]
use crate::envelope::watch::{maildir::WatchMaildirEnvelopes, WatchEnvelopes};
use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    envelope::{
        get::{maildir::GetMaildirEnvelope, GetEnvelope},
        list::{maildir::ListMaildirEnvelopes, ListEnvelopes},
    },
    flag::{
        add::{maildir::AddMaildirFlags, AddFlags},
        remove::{maildir::RemoveMaildirFlags, RemoveFlags},
        set::{maildir::SetMaildirFlags, SetFlags},
    },
    folder::{
        add::{maildir::AddMaildirFolder, AddFolder},
        delete::{maildir::DeleteMaildirFolder, DeleteFolder},
        expunge::{maildir::ExpungeMaildirFolder, ExpungeFolder},
        list::{maildir::ListMaildirFolders, ListFolders},
        FolderKind,
    },
    message::{
        add::{maildir::AddMaildirMessage, AddMessage},
        copy::{maildir::CopyMaildirMessages, CopyMessages},
        delete::{maildir::DeleteMaildirMessages, DeleteMessages},
        get::{maildir::GetMaildirMessages, GetMessages},
        peek::{maildir::PeekMaildirMessages, PeekMessages},
        r#move::{maildir::MoveMaildirMessages, MoveMessages},
        remove::{maildir::RemoveMaildirMessages, RemoveMessages},
    },
    AnyResult,
};

/// The Maildir backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`MaildirContextSync`].
pub struct MaildirContext {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The Maildir configuration.
    pub maildir_config: Arc<MaildirConfig>,

    /// The maildir instance.
    pub root: Maildirs,
}

impl MaildirContext {
    /// Create a maildir instance from a folder name.
    pub fn get_maildir_from_folder_alias(&self, folder: &str) -> Result<Maildir> {
        let folder = self.account_config.get_folder_alias(folder);

        // If the folder matches to the inbox folder kind, create a
        // maildir instance from the root folder.
        if self.maildir_config.maildirpp && FolderKind::matches_inbox(&folder) {
            return Ok(Maildir::from(try_shellexpand_path(self.root.path())?));
        }

        let mdir = self.root.get(folder)?;
        Ok(mdir)
    }
}

/// The sync version of the Maildir backend context.
///
/// This is just a Maildir session wrapped into a mutex, so the same
/// Maildir session can be shared and updated across multiple threads.
#[derive(Clone)]
pub struct MaildirContextSync {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The Maildir configuration.
    pub maildir_config: Arc<MaildirConfig>,

    inner: Arc<Mutex<MaildirContext>>,
}

impl Deref for MaildirContextSync {
    type Target = Arc<Mutex<MaildirContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BackendContext for MaildirContextSync {}

/// The Maildir backend context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaildirContextBuilder {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The Maildir configuration.
    pub mdir_config: Arc<MaildirConfig>,
}

impl MaildirContextBuilder {
    pub fn new(account_config: Arc<AccountConfig>, mdir_config: Arc<MaildirConfig>) -> Self {
        Self {
            account_config,
            mdir_config,
        }
    }

    pub fn expanded_root_dir(&self) -> PathBuf {
        shellexpand_path(&self.mdir_config.root_dir)
    }

    pub fn maildir(&self) -> Maildirs {
        Maildirs::new(self.expanded_root_dir()).with_maildirpp(self.mdir_config.maildirpp)
    }
}

#[cfg(feature = "sync")]
impl crate::sync::hash::SyncHash for MaildirContextBuilder {
    fn sync_hash(&self, state: &mut std::hash::DefaultHasher) {
        self.mdir_config.sync_hash(state);
    }
}

#[async_trait]
impl BackendContextBuilder for MaildirContextBuilder {
    type Context = MaildirContextSync;

    async fn configure(&mut self) -> AnyResult<()> {
        let mdir = self.maildir();

        if self.mdir_config.maildirpp {
            Maildir::from(mdir.path())
                .create_all()
                .map_err(|err| Error::CreateFolderStructureError(err, mdir.path().to_owned()))?;
        }

        Ok(())
    }

    fn check_configuration(&self) -> AnyResult<()> {
        match try_shellexpand_path(&self.mdir_config.root_dir) {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::CheckConfigurationInvalidPathError(err).into()),
        }
    }

    fn check_up(&self) -> Option<BackendFeature<Self::Context, dyn CheckUp>> {
        Some(Arc::new(CheckUpMaildir::some_new_boxed))
    }

    fn add_folder(&self) -> Option<BackendFeature<Self::Context, dyn AddFolder>> {
        Some(Arc::new(AddMaildirFolder::some_new_boxed))
    }

    fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
        Some(Arc::new(ListMaildirFolders::some_new_boxed))
    }

    fn expunge_folder(&self) -> Option<BackendFeature<Self::Context, dyn ExpungeFolder>> {
        Some(Arc::new(ExpungeMaildirFolder::some_new_boxed))
    }

    // TODO
    // fn purge_folder(&self) -> Option<BackendFeature<Self::Context, dyn PurgeFolder>> {
    //     Some(Arc::new(PurgeMaildirFolder::some_new_boxed))
    // }

    fn delete_folder(&self) -> Option<BackendFeature<Self::Context, dyn DeleteFolder>> {
        Some(Arc::new(DeleteMaildirFolder::some_new_boxed))
    }

    fn get_envelope(&self) -> Option<BackendFeature<Self::Context, dyn GetEnvelope>> {
        Some(Arc::new(GetMaildirEnvelope::some_new_boxed))
    }

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        Some(Arc::new(ListMaildirEnvelopes::some_new_boxed))
    }

    #[cfg(feature = "thread")]
    fn thread_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ThreadEnvelopes>> {
        Some(Arc::new(ThreadMaildirEnvelopes::some_new_boxed))
    }

    #[cfg(feature = "watch")]
    fn watch_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn WatchEnvelopes>> {
        Some(Arc::new(WatchMaildirEnvelopes::some_new_boxed))
    }

    fn add_flags(&self) -> Option<BackendFeature<Self::Context, dyn AddFlags>> {
        Some(Arc::new(AddMaildirFlags::some_new_boxed))
    }

    fn set_flags(&self) -> Option<BackendFeature<Self::Context, dyn SetFlags>> {
        Some(Arc::new(SetMaildirFlags::some_new_boxed))
    }

    fn remove_flags(&self) -> Option<BackendFeature<Self::Context, dyn RemoveFlags>> {
        Some(Arc::new(RemoveMaildirFlags::some_new_boxed))
    }

    fn add_message(&self) -> Option<BackendFeature<Self::Context, dyn AddMessage>> {
        Some(Arc::new(AddMaildirMessage::some_new_boxed))
    }

    fn peek_messages(&self) -> Option<BackendFeature<Self::Context, dyn PeekMessages>> {
        Some(Arc::new(PeekMaildirMessages::some_new_boxed))
    }

    fn get_messages(&self) -> Option<BackendFeature<Self::Context, dyn GetMessages>> {
        Some(Arc::new(GetMaildirMessages::some_new_boxed))
    }

    fn copy_messages(&self) -> Option<BackendFeature<Self::Context, dyn CopyMessages>> {
        Some(Arc::new(CopyMaildirMessages::some_new_boxed))
    }

    fn move_messages(&self) -> Option<BackendFeature<Self::Context, dyn MoveMessages>> {
        Some(Arc::new(MoveMaildirMessages::some_new_boxed))
    }

    fn delete_messages(&self) -> Option<BackendFeature<Self::Context, dyn DeleteMessages>> {
        Some(Arc::new(DeleteMaildirMessages::some_new_boxed))
    }

    fn remove_messages(&self) -> Option<BackendFeature<Self::Context, dyn RemoveMessages>> {
        Some(Arc::new(RemoveMaildirMessages::some_new_boxed))
    }

    async fn build(self) -> AnyResult<Self::Context> {
        info!("building new maildir context");

        let ctx = MaildirContext {
            account_config: self.account_config.clone(),
            maildir_config: self.mdir_config.clone(),
            root: self.maildir(),
        };

        Ok(MaildirContextSync {
            account_config: self.account_config,
            maildir_config: self.mdir_config,
            inner: Arc::new(Mutex::new(ctx)),
        })
    }
}

#[derive(Clone)]
pub struct CheckUpMaildir {
    pub ctx: MaildirContextSync,
}

impl CheckUpMaildir {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn CheckUp> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn CheckUp>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CheckUp for CheckUpMaildir {
    async fn check_up(&self) -> AnyResult<()> {
        // FIXME
        //
        // let ctx = self.ctx.lock().await;

        // ctx.root
        //     .list_cur()
        //     .try_for_each(|e| e.map(|_| ()))
        //     .map_err(Error::CheckUpCurrentDirectoryError)?;

        Ok(())
    }
}

/// URL-encode the given folder.
pub fn encode_folder(folder: impl AsRef<str>) -> String {
    urlencoding::encode(folder.as_ref()).to_string()
}

/// URL-decode the given folder.
pub fn decode_folder(folder: impl AsRef<str> + ToString) -> String {
    urlencoding::decode(folder.as_ref())
        .map(|folder| folder.to_string())
        .unwrap_or_else(|_| folder.to_string())
}
