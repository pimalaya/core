pub mod config;

use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use shellexpand_utils::{shellexpand_path, try_shellexpand_path};
use std::{ops::Deref, sync::Arc};
use tokio::sync::Mutex;

#[cfg(feature = "envelope-get")]
use crate::envelope::get::{maildir::GetMaildirEnvelope, GetEnvelope};
#[cfg(feature = "envelope-list")]
use crate::envelope::list::{maildir::ListMaildirEnvelopes, ListEnvelopes};
#[cfg(feature = "envelope-watch")]
use crate::envelope::watch::{maildir::WatchMaildirEnvelopes, WatchEnvelopes};
#[cfg(feature = "flag-add")]
use crate::flag::add::{maildir::AddMaildirFlags, AddFlags};
#[cfg(feature = "flag-remove")]
use crate::flag::remove::{maildir::RemoveMaildirFlags, RemoveFlags};
#[cfg(feature = "flag-set")]
use crate::flag::set::{maildir::SetMaildirFlags, SetFlags};
#[cfg(feature = "folder-add")]
use crate::folder::add::{maildir::AddMaildirFolder, AddFolder};
#[cfg(feature = "folder-delete")]
use crate::folder::delete::{maildir::DeleteMaildirFolder, DeleteFolder};
#[cfg(feature = "folder-expunge")]
use crate::folder::expunge::{maildir::ExpungeMaildirFolder, ExpungeFolder};
// TODO
// #[cfg(feature = "folder-purge")]
// use crate::folder::purge::{maildir::PurgeMaildirFolder, PurgeFolder};
#[cfg(feature = "folder-list")]
use crate::folder::list::{maildir::ListMaildirFolders, ListFolders};
#[cfg(feature = "message-add")]
use crate::message::add::{maildir::AddMaildirMessage, AddMessage};
#[cfg(feature = "message-copy")]
use crate::message::copy::{maildir::CopyMaildirMessages, CopyMessages};
#[cfg(feature = "message-delete")]
use crate::message::delete::{maildir::DeleteMaildirMessages, DeleteMessages};
#[cfg(feature = "message-get")]
use crate::message::get::{maildir::GetMaildirMessages, GetMessages};
#[cfg(feature = "message-peek")]
use crate::message::peek::{maildir::PeekMaildirMessages, PeekMessages};
#[cfg(feature = "message-move")]
use crate::message::r#move::{maildir::MoveMaildirMessages, MoveMessages};
use crate::{
    account::config::AccountConfig,
    backend::{BackendContext, BackendContextBuilder, BackendFeatureBuilder},
    folder::FolderKind,
    maildir, Result,
};

use self::config::MaildirConfig;

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
    pub root: Maildir,
}

impl MaildirContext {
    /// Create a maildir instance from a folder name.
    pub fn get_maildir_from_folder_name(&self, folder: &str) -> Result<Maildir> {
        // If the folder matches to the inbox folder kind, create a
        // maildir instance from the root folder.
        if FolderKind::matches_inbox(folder) {
            return try_shellexpand_path(self.root.path())
                .map(Maildir::from)
                .map_err(Into::into);
        }

        let folder = self.account_config.get_folder_alias(folder);

        // If the folder is a valid maildir path, create a maildir
        // instance from it. First check for absolute path…
        try_shellexpand_path(&folder)
            // then check for relative path to `maildir-dir`…
            .or_else(|_| try_shellexpand_path(self.root.path().join(&folder)))
            // TODO: should move to CLI
            // // and finally check for relative path to the current
            // // directory
            // .or_else(|_| {
            //     try_shellexpand_path(
            //         env::current_dir()
            //             .map_err(Error::GetCurrentFolderError)?
            //             .join(&folder),
            //     )
            // })
            .or_else(|_| {
                // Otherwise create a maildir instance from a maildir
                // subdirectory by adding a "." in front of the name
                // as described in the [spec].
                //
                // [spec]: http://www.courier-mta.org/imap/README.maildirquota.html
                let folder = maildir::encode_folder(&folder);
                try_shellexpand_path(self.root.path().join(format!(".{}", folder)))
            })
            .map(Maildir::from)
            .map_err(Into::into)
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
}

#[async_trait]
impl BackendContextBuilder for MaildirContextBuilder {
    type Context = MaildirContextSync;

    #[cfg(feature = "folder-add")]
    fn add_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFolder> {
        BackendFeatureBuilder::new(AddMaildirFolder::some_new_boxed)
    }

    #[cfg(feature = "folder-list")]
    fn list_folders(&self) -> BackendFeatureBuilder<Self::Context, dyn ListFolders> {
        BackendFeatureBuilder::new(ListMaildirFolders::some_new_boxed)
    }

    #[cfg(feature = "folder-expunge")]
    fn expunge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn ExpungeFolder> {
        BackendFeatureBuilder::new(ExpungeMaildirFolder::some_new_boxed)
    }

    // TODO
    // #[cfg(feature = "folder-purge")]
    // fn purge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn PurgeFolder> {
    //     BackendFeatureBuilder::new(PurgeMaildirFolder::some_new_boxed)
    // }

    #[cfg(feature = "folder-delete")]
    fn delete_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteFolder> {
        BackendFeatureBuilder::new(DeleteMaildirFolder::some_new_boxed)
    }

    #[cfg(feature = "envelope-list")]
    fn list_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn ListEnvelopes> {
        BackendFeatureBuilder::new(ListMaildirEnvelopes::some_new_boxed)
    }

    #[cfg(feature = "envelope-watch")]
    fn watch_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn WatchEnvelopes> {
        BackendFeatureBuilder::new(WatchMaildirEnvelopes::some_new_boxed)
    }

    #[cfg(feature = "envelope-get")]
    fn get_envelope(&self) -> BackendFeatureBuilder<Self::Context, dyn GetEnvelope> {
        BackendFeatureBuilder::new(GetMaildirEnvelope::some_new_boxed)
    }

    #[cfg(feature = "flag-add")]
    fn add_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFlags> {
        BackendFeatureBuilder::new(AddMaildirFlags::some_new_boxed)
    }

    #[cfg(feature = "flag-set")]
    fn set_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn SetFlags> {
        BackendFeatureBuilder::new(SetMaildirFlags::some_new_boxed)
    }

    #[cfg(feature = "flag-remove")]
    fn remove_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn RemoveFlags> {
        BackendFeatureBuilder::new(RemoveMaildirFlags::some_new_boxed)
    }

    #[cfg(feature = "message-add")]
    fn add_message(&self) -> BackendFeatureBuilder<Self::Context, dyn AddMessage> {
        BackendFeatureBuilder::new(AddMaildirMessage::some_new_boxed)
    }

    #[cfg(feature = "message-get")]
    fn get_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn GetMessages> {
        BackendFeatureBuilder::new(GetMaildirMessages::some_new_boxed)
    }

    #[cfg(feature = "message-peek")]
    fn peek_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn PeekMessages> {
        BackendFeatureBuilder::new(PeekMaildirMessages::some_new_boxed)
    }

    #[cfg(feature = "message-copy")]
    fn copy_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn CopyMessages> {
        BackendFeatureBuilder::new(CopyMaildirMessages::some_new_boxed)
    }

    #[cfg(feature = "message-move")]
    fn move_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn MoveMessages> {
        BackendFeatureBuilder::new(MoveMaildirMessages::some_new_boxed)
    }

    #[cfg(feature = "message-delete")]
    fn delete_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteMessages> {
        BackendFeatureBuilder::new(DeleteMaildirMessages::some_new_boxed)
    }

    async fn build(self) -> Result<Self::Context> {
        info!("building new maildir context");

        let path = shellexpand_path(&self.mdir_config.root_dir);

        let root = Maildir::from(path);
        root.create_dirs()?;

        let ctx = MaildirContext {
            account_config: self.account_config.clone(),
            maildir_config: self.mdir_config.clone(),
            root,
        };

        Ok(MaildirContextSync {
            account_config: self.account_config,
            maildir_config: self.mdir_config,
            inner: Arc::new(Mutex::new(ctx)),
        })
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
