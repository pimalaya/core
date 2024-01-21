pub mod config;

use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use shellexpand_utils::{shellexpand_path, try_shellexpand_path};
use std::{ops::Deref, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    account::config::AccountConfig, backend::BackendContextBuilder, folder::FolderKind, maildir,
    Result,
};

use self::config::MaildirConfig;

/// The Maildir backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`MaildirContextSync`].
pub struct MaildirContext {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The Maildir configuration.
    pub maildir_config: MaildirConfig,

    /// The maildir instance.
    pub root: Maildir,
}

impl MaildirContext {
    /// Create a maildir instance from a folder name.
    pub fn get_maildir_from_folder_name(&self, folder: &str) -> Result<Maildir> {
        // If the folder matches to the inbox folder kind, create a
        // maildir instance from the root folder.
        if FolderKind::matches_inbox(folder) {
            return try_shellexpand_path(self.root.path().to_owned())
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
    pub account_config: AccountConfig,

    /// The Maildir configuration.
    pub maildir_config: MaildirConfig,

    inner: Arc<Mutex<MaildirContext>>,
}

impl Deref for MaildirContextSync {
    type Target = Arc<Mutex<MaildirContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// The Maildir backend context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaildirContextBuilder {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The Maildir configuration.
    pub maildir_config: MaildirConfig,
}

impl MaildirContextBuilder {
    pub fn new(account_config: AccountConfig, maildir_config: MaildirConfig) -> Self {
        Self {
            account_config,
            maildir_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for MaildirContextBuilder {
    type Context = MaildirContextSync;

    async fn build(self) -> Result<Self::Context> {
        info!("building new maildir context");

        let path = shellexpand_path(&self.maildir_config.root_dir);

        let root = Maildir::from(path);
        root.create_dirs()?;

        let ctx = MaildirContext {
            account_config: self.account_config.clone(),
            maildir_config: self.maildir_config.clone(),
            root,
        };

        Ok(MaildirContextSync {
            account_config: self.account_config,
            maildir_config: self.maildir_config,
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
