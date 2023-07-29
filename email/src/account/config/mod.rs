//! Module dedicated to account configuration.
//!
//! This module contains the representation of the user's current
//! account configuration named [`AccountConfig`].

pub mod oauth2;
pub mod passwd;
pub mod pgp;

use dirs::data_dir;
use log::{debug, warn};
use mail_builder::headers::address::{Address, EmailAddress};
use pimalaya_email_tpl::TplInterpreter;
use shellexpand;
use std::{collections::HashMap, env, ffi::OsStr, fs, io, path::PathBuf, vec};
use thiserror::Error;

use crate::{
    backend::BackendConfig,
    email::config::{EmailHooks, EmailTextPlainFormat},
    folder::sync::FolderSyncStrategy,
    sender::SenderConfig,
    Result,
};

#[doc(inline)]
pub use self::{
    oauth2::{OAuth2Config, OAuth2Method, OAuth2Scopes},
    passwd::PasswdConfig,
    pgp::PgpConfig,
};

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIGNATURE_DELIM: &str = "-- \n";

pub const DEFAULT_INBOX_FOLDER: &str = "INBOX";
pub const DEFAULT_SENT_FOLDER: &str = "Sent";
pub const DEFAULT_DRAFTS_FOLDER: &str = "Drafts";
pub const DEFAULT_TRASH_FOLDER: &str = "Trash";

/// Errors related to account configuration.
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
    /// Represents the folder aliases hash map. There is 4 special
    /// aliases:
    /// - `inbox`: main folder containing incoming emails
    /// - `draft(s)`: folder containing draft emails
    /// - `sent`: folder containing emails sent
    /// - `trash`: folder containing trashed emails
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
    pub sync_folders_strategy: FolderSyncStrategy,

    /// The [Backend](crate::Backend) configuration.
    pub backend: BackendConfig,
    /// The [Sender](crate::Sender) configuration.
    pub sender: SenderConfig,

    /// The configuration related to PGP encryption.
    pub pgp: PgpConfig,
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
            pgp: Default::default(),
        }
    }
}

impl AccountConfig {
    /// Build a [`mail_builder::headers::address`] from a user account
    /// configuration. This is mostly used for building the `From`
    /// header by message builders.
    pub fn from(&self) -> Address {
        Address::Address(EmailAddress {
            name: self.display_name.clone().map(Into::into),
            email: self.email.clone().into(),
        })
    }

    /// Expand the downloads directory path. Falls back to temporary
    /// directory.
    pub fn downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .and_then(|dir| dir.to_str())
            .and_then(|dir| shellexpand::full(dir).ok())
            .map(|dir| PathBuf::from(dir.to_string()))
            .unwrap_or_else(env::temp_dir)
    }

    /// Wrapper around `downloads_dir()` and `rename_file_if_duplicate()`.
    pub fn download_fpath(&self, fname: impl AsRef<str>) -> Result<PathBuf> {
        let fpath = self.downloads_dir().join(fname.as_ref());
        rename_file_if_duplicate(&fpath, |path, _count| path.is_file())
    }

    /// Return the alias of the given folder if defined, otherwise
    /// return None.
    pub fn find_folder_alias(&self, folder: &str) -> Result<Option<String>> {
        let lowercase_folder = folder.trim().to_lowercase();

        let alias = match self.folder_aliases.get(&lowercase_folder) {
            None => None,
            Some(alias) => Some(shellexpand::full(alias).map(String::from).or_else(|err| {
                warn!("skipping shell expand for folder alias {alias}: {err}");
                debug!("skipping shell expand for folder alias {alias}: {err:?}");
                Result::Ok(alias.clone())
            })?),
        };

        Ok(alias)
    }

    /// Return the alias of the given folder if defined, otherwise
    /// return the folder itself. Then expand shell variables.
    pub fn get_folder_alias(&self, folder: &str) -> Result<String> {
        let lowercase_folder = folder.trim().to_lowercase();

        let alias = self
            .folder_aliases
            .get(&lowercase_folder)
            .map(String::as_str)
            .unwrap_or_else(|| match lowercase_folder.as_str() {
                "inbox" => DEFAULT_INBOX_FOLDER,
                "draft" | "drafts" => DEFAULT_DRAFTS_FOLDER,
                "sent" => DEFAULT_SENT_FOLDER,
                "trash" => DEFAULT_TRASH_FOLDER,
                _ => folder,
            });
        let alias = shellexpand::full(alias).map(String::from).or_else(|err| {
            warn!("skipping shell expand for folder alias {}: {}", alias, err);
            Result::Ok(alias.to_string())
        })?;

        debug!("folder alias for {folder}: {alias}");
        Ok(alias)
    }

    /// Return the inbox folder alias.
    pub fn inbox_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_INBOX_FOLDER)
    }

    /// Return the drafts folder alias.
    pub fn drafts_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_DRAFTS_FOLDER)
    }

    /// Return the sent folder alias.
    pub fn sent_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_SENT_FOLDER)
    }

    /// Return the trash folder alias.
    pub fn trash_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_TRASH_FOLDER)
    }

    /// Return the email listing page size if defined, otherwise
    /// return the default one.
    pub fn email_listing_page_size(&self) -> usize {
        self.email_listing_page_size.unwrap_or(DEFAULT_PAGE_SIZE)
    }

    /// Return the email reading headers if defined, otherwise return
    /// the default ones.
    pub fn email_reading_headers(&self) -> Vec<String> {
        self.email_reading_headers
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| vec!["From".into(), "To".into(), "Cc".into(), "Subject".into()])
    }

    /// Return the email writing headers if defined, otherwise return
    /// the default ones.
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

    /// Return the full signature, including the delimiter.
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

    /// Return `true` if the sync directory already exists.
    pub fn sync_dir_exists(&self) -> bool {
        match self.sync_dir.as_ref() {
            Some(dir) => dir.is_dir(),
            None => data_dir()
                .map(|dir| dir.join("himalaya").join(&self.name).is_dir())
                .unwrap_or_default(),
        }
    }

    /// Return `true` if the sync directory already exists and if the
    /// sync feature is enabled.
    pub fn sync(&self) -> bool {
        self.sync && self.sync_dir_exists()
    }

    /// Return the sync directory if exist, otherwise create it.
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

    /// Open a SQLite connection to the synchronization database.
    pub fn sync_db_builder(&self) -> Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(self.sync_dir()?.join(".sync.sqlite"))
            .map_err(Error::BuildSyncDatabaseError)?;
        Ok(conn)
    }

    /// Generate a template interpreter with prefilled options from
    /// the current user account configuration.
    pub fn generate_tpl_interpreter(&self) -> TplInterpreter {
        TplInterpreter::new()
            .with_pgp_decrypt(self.pgp.clone())
            .with_pgp_verify(self.pgp.clone())
            .with_save_attachments_dir(self.downloads_dir())
    }

    /// Return the email listing datetime format, otherwise return the
    /// default one.
    pub fn email_listing_datetime_fmt(&self) -> String {
        self.email_listing_datetime_fmt
            .clone()
            .unwrap_or(String::from("%F %R%:z"))
    }

    /// Return the email listing datetime local timezone option,
    /// otherwise return the default one.
    pub fn email_listing_datetime_local_tz(&self) -> bool {
        self.email_listing_datetime_local_tz.unwrap_or_default()
    }
}

/// Rename duplicated file by adding a auto-incremented counter
/// suffix.
///
/// Helper that check if the given file path already exists: if so,
/// creates a new path with an auto-incremented integer suffix and
/// returs it, otherwise returs the original file path.
pub(crate) fn rename_file_if_duplicate(
    original_fpath: &PathBuf,
    is_file: impl Fn(&PathBuf, u8) -> bool,
) -> Result<PathBuf> {
    let mut count = 0;
    let fext = original_fpath
        .extension()
        .and_then(OsStr::to_str)
        .map(|fext| String::from(".") + fext)
        .unwrap_or_default();
    let mut fpath = original_fpath.clone();

    while is_file(&fpath, count) {
        count += 1;
        fpath.set_file_name(OsStr::new(
            &original_fpath
                .file_stem()
                .and_then(OsStr::to_str)
                .map(|fstem| format!("{}_{}{}", fstem, count, fext))
                .ok_or_else(|| Error::ParseDownloadFileNameError(fpath.to_owned()))?,
        ));
    }

    Ok(fpath)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn rename_file_if_duplicate() {
        let path = PathBuf::from("downloads/file.ext");

        // when file path is unique
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, _| false),
            Ok(path) if path == PathBuf::from("downloads/file.ext")
        ));

        // when 1 file path already exist
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count <  1),
            Ok(path) if path == PathBuf::from("downloads/file_1.ext")
        ));

        // when 5 file paths already exist
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5.ext")
        ));

        // when file path has no extension
        let path = PathBuf::from("downloads/file");
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5")
        ));

        // when file path has 2 extensions
        let path = PathBuf::from("downloads/file.ext.ext2");
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file.ext_5.ext2")
        ));
    }
}
