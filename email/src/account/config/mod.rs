//! Module dedicated to account configuration.
//!
//! This module contains the representation of the user's current
//! account configuration named [`AccountConfig`].

pub mod oauth2;
pub mod passwd;
#[cfg(feature = "pgp")]
pub mod pgp;

use dirs::data_dir;
use log::{debug, warn};
use mail_builder::headers::address::{Address, EmailAddress};
use mml::MimeInterpreterBuilder;
use shellexpand_utils::{shellexpand_str, try_shellexpand_path};
use std::{collections::HashMap, env, ffi::OsStr, fs, io, path::PathBuf, vec};
use thiserror::Error;

use crate::{
    backend::BackendConfig,
    boxed_err,
    email::config::{EmailHooks, EmailTextPlainFormat},
    folder::sync::FolderSyncStrategy,
    Result,
};

#[cfg(feature = "pgp-commands")]
#[doc(inline)]
pub use self::pgp::CmdsPgpConfig;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use self::pgp::GpgConfig;
#[cfg(feature = "pgp")]
#[doc(inline)]
pub use self::pgp::PgpConfig;
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use self::pgp::{NativePgpConfig, NativePgpSecretKey, SignedPublicKey, SignedSecretKey};
#[doc(inline)]
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
    pub email_sending_save_copy: Option<bool>,
    /// Represents the email hooks.
    pub email_hooks: EmailHooks,

    /// Enables the synchronization of this account with a local
    /// Maildir backend.
    pub sync: Option<bool>,
    /// Custom root directory where the Maildir cache is
    /// saved. Defaults to `$XDG_DATA_HOME/himalaya/<account-name>`.
    pub sync_dir: Option<PathBuf>,
    /// Represents the synchronization strategy to use for folders.
    pub sync_folders_strategy: FolderSyncStrategy,

    pub backends: HashMap<String, BackendConfig>,

    /// The configuration related to PGP encryption.
    #[cfg(feature = "pgp")]
    pub pgp: PgpConfig,
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
        match self.downloads_dir.as_ref() {
            Some(dir) => try_shellexpand_path(dir).unwrap_or_else(|err| {
                warn!("cannot expand downloads dir, falling back to tmp: {err}");
                debug!("cannot expand downloads dir: {err:?}");
                env::temp_dir()
            }),
            None => {
                warn!("downloads dir not defined, falling back to tmp");
                env::temp_dir()
            }
        }
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

        let alias = self
            .folder_aliases
            .get(&lowercase_folder)
            .map(shellexpand_str);

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
            .map(shellexpand_str)
            .unwrap_or_else(|| match lowercase_folder.as_str() {
                "inbox" => DEFAULT_INBOX_FOLDER.to_owned(),
                "draft" | "drafts" => DEFAULT_DRAFTS_FOLDER.to_owned(),
                "sent" => DEFAULT_SENT_FOLDER.to_owned(),
                "trash" => DEFAULT_TRASH_FOLDER.to_owned(),
                _ => shellexpand_str(folder),
            });

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
        let signature = signature.map(|path_or_raw| {
            let signature = try_shellexpand_path(path_or_raw)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
                .and_then(|path| fs::read_to_string(path))
                .unwrap_or_else(|err| {
                    warn!("cannot read signature from path: {err}");
                    debug!("cannot read signature from path: {err:?}");
                    shellexpand_str(path_or_raw)
                });
            format!("{}{}", delim, signature.trim())
        });

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
        matches!(self.sync, Some(true)) && self.sync_dir_exists()
    }

    /// Return the sync directory if exist, otherwise create it.
    pub fn sync_dir(&self) -> Result<PathBuf> {
        match self.sync_dir.as_ref().filter(|dir| dir.is_dir()) {
            Some(dir) => Ok(dir.clone()),
            None => {
                warn!("sync dir not set or invalid, falling back to $XDG_DATA_HOME/himalaya");
                let sync_dir = data_dir()
                    .map(|dir| dir.join("himalaya"))
                    .ok_or_else(|| boxed_err(Error::GetXdgDataDirError))?
                    .join(&self.name);
                fs::create_dir_all(&sync_dir)
                    .map_err(|err| boxed_err(Error::CreateXdgDataDirsError(err)))?;
                Ok(sync_dir)
            }
        }
    }

    /// Open a SQLite connection to the synchronization database.
    pub fn sync_db_builder(&self) -> Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(self.sync_dir()?.join(".sync.sqlite"))
            .map_err(|err| boxed_err(Error::BuildSyncDatabaseError(err)))?;
        Ok(conn)
    }

    /// Generate a template interpreter with prefilled options from
    /// the current user account configuration.
    pub fn generate_tpl_interpreter(&self) -> MimeInterpreterBuilder {
        let builder = MimeInterpreterBuilder::new().with_save_attachments_dir(self.downloads_dir());

        #[cfg(feature = "pgp")]
        let builder = builder.with_pgp(self.pgp.clone());

        builder
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
                .ok_or_else(|| boxed_err(Error::ParseDownloadFileNameError(fpath.to_owned())))?,
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
