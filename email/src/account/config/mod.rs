//! Module dedicated to account configuration.
//!
//! This module contains the representation of the user's current
//! account configuration named [`AccountConfig`].

pub mod oauth2;
pub mod passwd;

use dirs::data_dir;
use log::warn;
use mail_builder::headers::address::{Address, EmailAddress};
use pimalaya_email_tpl::TplInterpreter;
use pimalaya_process::Cmd;
use shellexpand;
use std::{collections::HashMap, env, ffi::OsStr, fs, io, path::PathBuf, vec};
use thiserror::Error;

use crate::{
    folder::sync::FolderSyncStrategy as SyncFoldersStrategy, BackendConfig, EmailHooks,
    EmailTextPlainFormat, Result, SenderConfig,
};

pub use self::{
    oauth2::{OAuth2Config, OAuth2Method, OAuth2Scopes},
    passwd::PasswdConfig,
};

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIGNATURE_DELIM: &str = "-- \n";

pub const DEFAULT_INBOX_FOLDER: &str = "INBOX";
pub const DEFAULT_SENT_FOLDER: &str = "Sent";
pub const DEFAULT_DRAFTS_FOLDER: &str = "Drafts";
pub const DEFAULT_TRASH_FOLDER: &str = "Trash";

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open the synchronization database")]
    BuildSyncDatabaseError(#[source] rusqlite::Error),
    #[error("cannot parse download file name from {0}")]
    ParseDownloadFileNameError(PathBuf),
    #[error("cannot get sync directory from XDG_DATA_HOME")]
    GetXdgDataDirError,
    #[error("cannot create sync directories")]
    CreateXdgDataDirsError(#[source] io::Error),
}

/// The user's account configuration.
///
/// It represents everything that the user can customize for a given
/// account. It is the main configuration used by all other
/// modules. Usually, it serves as a reference for building config
/// file structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountConfig {
    /// The name of the user account. It serves as an unique
    /// identifier for a given configuration.
    pub name: String,
    /// The email address of the user account.
    pub email: String,
    /// The display name of the user. It usually corresponds to the
    /// full name of the user.
    pub display_name: Option<String>,
    /// The email signature delimiter of the user signature. Defaults
    /// to `-- \n`.
    pub signature_delim: Option<String>,
    /// Represents the email signature of the user. It can be either a
    /// path to a file (usually `~/.signature`) or a raw string.
    pub signature: Option<String>,
    /// Represents the downloads directory. It is used for downloading
    /// attachments.
    pub downloads_dir: Option<PathBuf>,

    /// Represents the page size when listing folders. A page size of
    /// 0 disables the pagination and shows all available folders.
    pub folder_listing_page_size: Option<usize>,
    /// Represents the folder aliases hash map. There is 3 special
    /// aliases:
    /// - `inbox`: main folder containing incoming emails
    /// - `draft(s)`: folder containing draft emails
    /// - `sent`: folder containing emails sent
    pub folder_aliases: HashMap<String, String>,

    /// Represents the page size when listing envelopes. A page size
    /// of 0 disables the pagination and shows all available
    /// envelopes.
    pub email_listing_page_size: Option<usize>,
    /// Custom format for displaying envelopes date. See
    /// [`chrono::format::strftime`] for supported formats. Defaults
    /// to `%F %R%:z`.
    pub email_listing_datetime_fmt: Option<String>,
    /// If `true`, transform envelopes date timezone into the user's
    /// local one. For example, if the user's local timezone is UTC,
    /// the envelope date `2023-06-15T09:00:00+02:00` becomes
    /// `2023-06-15T07:00:00-00:00`.
    pub email_listing_datetime_local_tz: Option<bool>,

    /// Represents headers visible at the top of email messages when
    /// reading them.
    pub email_reading_headers: Option<Vec<String>>,
    /// Represents the text/plain format as defined in the
    /// [RFC 2646](https://www.ietf.org/rfc/rfc2646.txt).
    pub email_reading_format: EmailTextPlainFormat,
    /// Represents the PGP command used to verify an email.
    pub email_reading_verify_cmd: Option<Cmd>,
    /// Represents the PGP command used to decrypt an email.
    pub email_reading_decrypt_cmd: Option<Cmd>,
    /// Represents the PGP command used to sign an email.
    pub email_writing_sign_cmd: Option<Cmd>,
    /// Represents the PGP command used to encrypt an email.
    pub email_writing_encrypt_cmd: Option<Cmd>,
    /// Represents headers visible at the top of emails when writing
    /// them (new/reply/forward).
    pub email_writing_headers: Option<Vec<String>>,
    /// Should save a copy of the email being sent in the sent
    /// folder. The sent folder can be customized using
    /// `folder_aliases`. Knowing that 1) saving an email is done by
    /// the [Backend](crate::Backend), 2) sending an email is done by
    /// the [Sender](crate::Sender), and 3) both have no relation
    /// together, it is the library user's responsibility to check
    /// this option and to save the copy of the sent email.
    ///
    /// ```rust,ignore
    /// AccountConfig {
    ///     folder_aliases: HashMap::from_iter([("sent", "MyCustomSent")]),
    ///     email_sending_save_copy: true,
    ///     ..Default::default()
    /// };
    /// ```
    pub email_sending_save_copy: bool,
    /// Represents the email hooks.
    pub email_hooks: EmailHooks,

    /// Enables the synchronization of this account with a local
    /// Maildir backend.
    pub sync: bool,
    /// Custom root directory where the Maildir cache is
    /// saved. Defaults to `$XDG_DATA_HOME/himalaya/<account-name>`.
    pub sync_dir: Option<PathBuf>,
    /// Represents the synchronization strategy to use for folders.
    pub sync_folders_strategy: SyncFoldersStrategy,

    /// The [Backend](crate::Backend) configuration.
    pub backend: BackendConfig,
    /// The [Sender](crate::Sender) configuration.
    pub sender: SenderConfig,
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            name: Default::default(),
            email: Default::default(),
            display_name: Default::default(),
            signature_delim: Default::default(),
            signature: Default::default(),
            downloads_dir: Default::default(),
            folder_listing_page_size: Default::default(),
            folder_aliases: Default::default(),
            email_listing_page_size: Default::default(),
            email_listing_datetime_fmt: Default::default(),
            email_listing_datetime_local_tz: Default::default(),
            email_reading_headers: Default::default(),
            email_reading_format: Default::default(),
            email_reading_verify_cmd: Default::default(),
            email_reading_decrypt_cmd: Default::default(),
            email_writing_sign_cmd: Default::default(),
            email_writing_encrypt_cmd: Default::default(),
            email_writing_headers: Default::default(),
            // NOTE: manually implementing the Default trait just for
            // this field:
            email_sending_save_copy: true,
            email_hooks: Default::default(),
            sync: Default::default(),
            sync_dir: Default::default(),
            sync_folders_strategy: Default::default(),
            backend: Default::default(),
            sender: Default::default(),
        }
    }
}

impl AccountConfig {
    pub fn addr(&self) -> Address {
        Address::Address(EmailAddress {
            name: self.display_name.clone().map(Into::into),
            email: self.email.clone().into(),
        })
    }

    /// Gets the downloads directory path.
    pub fn downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .and_then(|dir| dir.to_str())
            .and_then(|dir| shellexpand::full(dir).ok())
            .map(|dir| PathBuf::from(dir.to_string()))
            .unwrap_or_else(env::temp_dir)
    }

    /// Gets the download path from a file name.
    pub fn get_download_file_path<S: AsRef<str>>(&self, file_name: S) -> Result<PathBuf> {
        let file_path = self.downloads_dir().join(file_name.as_ref());
        self.get_unique_download_file_path(&file_path, |path, _count| path.is_file())
    }

    /// Gets the unique download path from a file name by adding
    /// suffixes in case of name conflicts.
    pub(crate) fn get_unique_download_file_path(
        &self,
        original_file_path: &PathBuf,
        is_file: impl Fn(&PathBuf, u8) -> bool,
    ) -> Result<PathBuf> {
        let mut count = 0;
        let file_ext = original_file_path
            .extension()
            .and_then(OsStr::to_str)
            .map(|fext| String::from(".") + fext)
            .unwrap_or_default();
        let mut file_path = original_file_path.clone();

        while is_file(&file_path, count) {
            count += 1;
            file_path.set_file_name(OsStr::new(
                &original_file_path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .map(|fstem| format!("{}_{}{}", fstem, count, file_ext))
                    .ok_or_else(|| Error::ParseDownloadFileNameError(file_path.to_owned()))?,
            ));
        }

        Ok(file_path)
    }

    /// Gets the alias of the given folder if exists, otherwise
    /// returns the folder itself. Also tries to expand shell
    /// variables.
    pub fn folder_alias(&self, folder: &str) -> Result<String> {
        let lowercase_folder = folder.trim().to_lowercase();

        let alias = self
            .folder_aliases
            .get(&lowercase_folder)
            .map(String::as_str)
            .unwrap_or_else(|| match lowercase_folder.as_str() {
                "inbox" => DEFAULT_INBOX_FOLDER,
                "draft" | "drafts" => DEFAULT_DRAFTS_FOLDER,
                "sent" => DEFAULT_SENT_FOLDER,
                _ => folder,
            });
        let alias = shellexpand::full(alias).map(String::from).or_else(|err| {
            warn!("skipping shell expand for folder alias {}: {}", alias, err);
            Result::Ok(alias.to_string())
        })?;

        Ok(alias)
    }

    pub fn inbox_folder_alias(&self) -> Result<String> {
        self.folder_alias(DEFAULT_INBOX_FOLDER)
    }

    pub fn drafts_folder_alias(&self) -> Result<String> {
        self.folder_alias(DEFAULT_DRAFTS_FOLDER)
    }

    pub fn sent_folder_alias(&self) -> Result<String> {
        self.folder_alias(DEFAULT_SENT_FOLDER)
    }

    pub fn trash_folder_alias(&self) -> Result<String> {
        self.folder_alias(DEFAULT_TRASH_FOLDER)
    }

    pub fn email_listing_page_size(&self) -> usize {
        self.email_listing_page_size.unwrap_or(DEFAULT_PAGE_SIZE)
    }

    pub fn email_reading_headers(&self) -> Vec<String> {
        self.email_reading_headers
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| vec!["From".into(), "To".into(), "Cc".into(), "Subject".into()])
    }

    pub fn email_writing_headers(&self) -> Vec<String> {
        self.email_writing_headers
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                vec![
                    "From".into(),
                    "To".into(),
                    "In-Reply-To".into(),
                    "Cc".into(),
                    "Subject".into(),
                ]
            })
    }

    pub fn signature(&self) -> Result<Option<String>> {
        let delim = self
            .signature_delim
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_SIGNATURE_DELIM);

        let signature = self.signature.as_ref();
        let signature = signature
            .and_then(|sig| shellexpand::full(sig).ok())
            .map(String::from)
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| signature.map(ToOwned::to_owned))
            .map(|sig| format!("{}{}", delim, sig.trim()));

        Ok(signature)
    }

    pub fn sync(&self) -> bool {
        self.sync
            && match self.sync_dir.as_ref() {
                Some(dir) => dir.is_dir(),
                None => data_dir()
                    .map(|dir| dir.join("himalaya").join(&self.name).is_dir())
                    .unwrap_or_default(),
            }
    }

    pub fn sync_dir_exists(&self) -> bool {
        match self.sync_dir.as_ref() {
            Some(dir) => dir.is_dir(),
            None => data_dir()
                .map(|dir| dir.join("himalaya").join(&self.name).is_dir())
                .unwrap_or_default(),
        }
    }

    pub fn sync_dir(&self) -> Result<PathBuf> {
        match self.sync_dir.as_ref().filter(|dir| dir.is_dir()) {
            Some(dir) => Ok(dir.clone()),
            None => {
                warn!("sync dir not set or invalid, falling back to $XDG_DATA_HOME/himalaya");
                let sync_dir = data_dir()
                    .map(|dir| dir.join("himalaya"))
                    .ok_or(Error::GetXdgDataDirError)?
                    .join(&self.name);
                fs::create_dir_all(&sync_dir).map_err(Error::CreateXdgDataDirsError)?;
                Ok(sync_dir)
            }
        }
    }

    pub fn sync_db_builder(&self) -> Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(self.sync_dir()?.join(".sync.sqlite"))
            .map_err(Error::BuildSyncDatabaseError)?;
        Ok(conn)
    }

    pub fn generate_tpl_interpreter(&self) -> TplInterpreter {
        TplInterpreter::new()
            .some_pgp_decrypt_cmd(self.email_reading_decrypt_cmd.clone())
            .some_pgp_verify_cmd(self.email_reading_verify_cmd.clone())
            .save_attachments_dir(self.downloads_dir())
    }

    pub fn email_listing_datetime_fmt(&self) -> String {
        self.email_listing_datetime_fmt
            .clone()
            .unwrap_or(String::from("%F %R%:z"))
    }

    pub fn email_listing_datetime_local_tz(&self) -> bool {
        self.email_listing_datetime_local_tz.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::AccountConfig;

    #[test]
    fn unique_download_file_path() {
        let config = AccountConfig::default();
        let path = PathBuf::from("downloads/file.ext");

        // when file path is unique
        assert!(matches!(
            config.get_unique_download_file_path(&path, |_, _| false),
            Ok(path) if path == PathBuf::from("downloads/file.ext")
        ));

        // when 1 file path already exist
        assert!(matches!(
            config.get_unique_download_file_path(&path, |_, count| count <  1),
            Ok(path) if path == PathBuf::from("downloads/file_1.ext")
        ));

        // when 5 file paths already exist
        assert!(matches!(
            config.get_unique_download_file_path(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5.ext")
        ));

        // when file path has no extension
        let path = PathBuf::from("downloads/file");
        assert!(matches!(
            config.get_unique_download_file_path(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5")
        ));

        // when file path has 2 extensions
        let path = PathBuf::from("downloads/file.ext.ext2");
        assert!(matches!(
            config.get_unique_download_file_path(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file.ext_5.ext2")
        ));
    }
}
