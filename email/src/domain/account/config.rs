//! Account config module.
//!
//! This module contains the representation of the user's current
//! account configuration.

use dirs::data_dir;
use lettre::{address::AddressError, message::Mailbox};
use log::warn;
use shellexpand;
use std::{collections::HashMap, env, ffi::OsStr, fs, io, path::PathBuf, result};
use thiserror::Error;

use crate::{
    folder::sync::Strategy as SyncFoldersStrategy, process, EmailHooks, EmailSender,
    EmailTextPlainFormat,
};

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIGNATURE_DELIM: &str = "-- \n";

pub const DEFAULT_INBOX_FOLDER: &str = "INBOX";
pub const DEFAULT_SENT_FOLDER: &str = "Sent";
pub const DEFAULT_DRAFTS_FOLDER: &str = "Drafts";
pub const DEFAULT_TRASH_FOLDER: &str = "Trash";

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot encrypt file using pgp")]
    EncryptFileError(#[source] process::Error),
    #[error("cannot find encrypt file command from config file")]
    EncryptFileMissingCmdError,
    #[error("cannot decrypt file using pgp")]
    DecryptFileError(#[source] process::Error),
    #[error("cannot find decrypt file command from config file")]
    DecryptFileMissingCmdError,
    #[error("cannot parse account address {0}")]
    ParseAccountAddrError(#[source] mailparse::MailParseError, String),
    #[error("cannot find account address in {0}")]
    ParseAccountAddrNotFoundError(String),
    #[error("cannot parse download file name from {0}")]
    ParseDownloadFileNameError(PathBuf),
    #[error("cannot parse address from config")]
    ParseAddressError(#[source] AddressError),

    #[error("cannot get sync directory from XDG_DATA_HOME")]
    GetXdgDataDirError,
    #[error("cannot create sync directories")]
    CreateXdgDataDirsError(#[source] io::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the configuration of the user account.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct AccountConfig {
    /// Represents the name of the current user account.
    pub name: String,
    /// Represents the email address of the user.
    pub email: String,
    /// Represents the display name of the user.
    pub display_name: Option<String>,
    /// Represents the email signature delimiter of the user.
    pub signature_delim: Option<String>,
    /// Represents the email signature of the user.
    pub signature: Option<String>,
    /// Represents the downloads directory (mostly for attachments).
    pub downloads_dir: Option<PathBuf>,

    /// Represents the page size when listing folders.
    pub folder_listing_page_size: Option<usize>,
    /// Represents the folder aliases hash map.
    pub folder_aliases: HashMap<String, String>,

    /// Represents the page size when listing emails.
    pub email_listing_page_size: Option<usize>,
    /// Represents headers visible at the top of emails when reading
    /// them.
    pub email_reading_headers: Option<Vec<String>>,
    /// Represents the text/plain format as defined in the
    /// [RFC 2646](https://www.ietf.org/rfc/rfc2646.txt).
    pub email_reading_format: EmailTextPlainFormat,
    /// Represents the command used to verify an email.
    pub email_reading_verify_cmd: Option<String>,
    /// Represents the command used to decrypt an email.
    pub email_reading_decrypt_cmd: Option<String>,
    /// Represents the command used to sign an email.
    pub email_writing_sign_cmd: Option<String>,
    /// Represents the command used to encrypt an email.
    pub email_writing_encrypt_cmd: Option<String>,
    /// Represents headers visible at the top of emails when writing
    /// them (new/reply/forward).
    pub email_writing_headers: Option<Vec<String>>,
    /// Represents the email sender provider.
    pub email_sender: EmailSender,
    /// Represents the email hooks.
    pub email_hooks: EmailHooks,

    /// Enables the automatic synchronization of this account with a
    /// local Maildir backend.
    pub sync: bool,
    /// Customizes the root directory where the Maildir cache is
    /// saved. Defaults to `$XDG_DATA_HOME/himalaya/<account-name>`.
    pub sync_dir: Option<PathBuf>,
    /// Represents the synchronization strategy to use for folders.
    pub sync_folders_strategy: SyncFoldersStrategy,
}

impl AccountConfig {
    /// Builds the full [RFC 2822] compliant user email address.
    ///
    /// [RFC 2822]: https://www.rfc-editor.org/rfc/rfc2822
    pub fn addr(&self) -> Result<Mailbox> {
        Ok(Mailbox::new(
            self.display_name.clone(),
            self.email.parse().map_err(Error::ParseAddressError)?,
        ))
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
            Ok(alias.to_string())
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
            .unwrap_or_default()
    }

    pub fn email_writing_headers<I: ToString, H: IntoIterator<Item = I>>(
        &self,
        more_headers: H,
    ) -> Vec<String> {
        let mut headers = self
            .email_reading_headers
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        headers.extend(
            more_headers
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );
        headers
    }

    pub fn signature(&self) -> Result<Option<String>> {
        let delim = self
            .signature_delim
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(DEFAULT_SIGNATURE_DELIM);
        let signature = self.signature.as_ref();

        Ok(signature
            .and_then(|sig| shellexpand::full(sig).ok())
            .map(String::from)
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| signature.map(ToOwned::to_owned))
            .map(|sig| format!("{}{}", delim, sig)))
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
}

#[cfg(test)]
mod account_config {
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
