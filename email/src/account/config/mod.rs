//! Module dedicated to account configuration.
//!
//! This module contains the representation of the user's current
//! account configuration named [`AccountConfig`].

pub mod oauth2;
pub mod passwd;
#[cfg(feature = "pgp")]
pub mod pgp;

use dirs::data_dir;
use log::debug;
use mail_builder::headers::address::{Address, EmailAddress};
use mml::MimeInterpreterBuilder;
use process::Cmd;
use serde::{Deserialize, Serialize};
use shellexpand_utils::{shellexpand_path, shellexpand_str, try_shellexpand_path};
use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    vec,
};
use thiserror::Error;

use crate::{
    email::config::EmailTextPlainFormat,
    envelope::config::EnvelopeConfig,
    folder::{config::FolderConfig, sync::FolderSyncStrategy},
    message::config::MessageConfig,
    Result,
};

#[cfg(feature = "pgp")]
use self::pgp::PgpConfig;

use super::sync::config::SyncConfig;

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
    #[error("cannot get file name from path {0}")]
    GetFileNameFromPathError(PathBuf),
}

/// The user's account configuration.
///
/// It represents everything that the user can customize for a given
/// account. It is the main configuration used by all other
/// modules. Usually, it serves as a reference for building config
/// file structure.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountConfig {
    /// The name of the user account.
    ///
    /// The account name is used as an unique identifier for a given
    /// configuration.
    pub name: String,

    /// The email address of the user account.
    pub email: String,

    /// The display name of the user.
    ///
    /// It usually corresponds to the full name of the user.
    pub display_name: Option<String>,

    /// The email signature of the user.
    ///
    /// It can be either a path to a file (usually `~/.signature`) or
    /// a raw string.
    pub signature: Option<String>,

    /// The email signature delimiter of the user signature.
    ///
    /// Defaults to `-- \n`.
    pub signature_delim: Option<String>,

    /// The downloads directory.
    ///
    /// It is mostly used for downloading messages
    /// attachments. Defaults to the system temporary directory
    /// (usually `/tmp`).
    pub downloads_dir: Option<PathBuf>,

    /// The account synchronization configuration.
    pub sync: Option<SyncConfig>,

    /// The folder configuration.
    pub folder: Option<FolderConfig>,

    /// The envelope configuration.
    pub envelope: Option<EnvelopeConfig>,

    /// The message configuration.
    pub message: Option<MessageConfig>,

    /// The PGP configuration.
    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,
}

impl AccountConfig {
    /// Find the full signature, including the delimiter.
    pub fn find_full_signature(&self) -> Result<Option<String>> {
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
                    debug!("cannot read signature from path: {err}");
                    debug!("{err:?}");
                    shellexpand_str(path_or_raw)
                });
            format!("{}{}", delim, signature.trim())
        });

        Ok(signature)
    }

    /// Get then expand the downloads directory path.
    ///
    /// Falls back to the system's temporary directory.
    pub fn get_downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .map(shellexpand_path)
            .unwrap_or_else(env::temp_dir)
    }

    /// Build the downloadable version of the given path.
    ///
    /// The aim of this helper is to build a safe download path for a
    /// given path.
    ///
    /// First, only the file name of the give path is taken in order
    /// to prevent any interaction outside of the downloads directory.
    ///
    /// Then, a suffix may be added to the final path if it already
    /// exists on the filesystem in order to prevent any overriding or
    /// data loss.
    pub fn get_download_file_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = path.as_ref();

        let file_name = path
            .file_name()
            .ok_or_else(|| Error::GetFileNameFromPathError(path.to_owned()))?;

        let final_path = self.get_downloads_dir().join(file_name);

        rename_file_if_duplicate(&final_path, |path, _count| path.is_file())
    }

    /// Return `true` if the synchronization is enabled.
    pub fn is_sync_enabled(&self) -> bool {
        self.sync
            .as_ref()
            .and_then(|c| c.enable)
            .unwrap_or_default()
    }

    /// Return `true` if the synchronization directory already exists.
    pub fn does_sync_dir_exist(&self) -> bool {
        match self.sync.as_ref().and_then(|c| c.dir.as_ref()) {
            Some(dir) => try_shellexpand_path(dir).is_ok(),
            None => data_dir()
                .map(|dir| dir.join("himalaya").join(&self.name).is_dir())
                .unwrap_or_default(),
        }
    }

    /// Return `true` if the synchronization is enabled AND if the
    /// sync directory exists.
    pub fn is_sync_usable(&self) -> bool {
        self.is_sync_enabled() && self.does_sync_dir_exist()
    }

    /// Get the synchronization directory if exist, otherwise create
    /// it.
    pub fn get_sync_dir(&self) -> Result<PathBuf> {
        match self.sync.as_ref().and_then(|c| c.dir.as_ref()) {
            Some(dir) => {
                let sync_dir = shellexpand_path(dir);
                if !sync_dir.is_dir() {
                    fs::create_dir_all(&sync_dir).map_err(Error::CreateXdgDataDirsError)?;
                }
                Ok(sync_dir)
            }
            None => {
                debug!("sync dir not set or invalid, falling back to $XDG_DATA_HOME/himalaya");
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
    pub fn get_sync_db_conn(&self) -> Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(self.get_sync_dir()?.join(".sync.sqlite"))
            .map_err(Error::BuildSyncDatabaseError)?;
        Ok(conn)
    }

    /// Find the alias of the given folder.
    ///
    /// The alias is also shell expanded.
    pub fn find_folder_alias(&self, folder: &str) -> Result<Option<String>> {
        let lowercase_folder = folder.trim().to_lowercase();

        let alias = self
            .folder
            .as_ref()
            .and_then(|c| c.aliases.as_ref())
            .and_then(|aliases| aliases.get(&lowercase_folder).map(shellexpand_str));

        Ok(alias)
    }

    /// Find the alias of the given folder, otherwise return the given
    /// folder itself.
    pub fn get_folder_alias(&self, folder: &str) -> Result<String> {
        let alias = self.find_folder_alias(folder)?.unwrap_or_else(|| {
            match folder.trim().to_lowercase().as_str() {
                "inbox" => DEFAULT_INBOX_FOLDER.to_owned(),
                "draft" | "drafts" => DEFAULT_DRAFTS_FOLDER.to_owned(),
                "sent" => DEFAULT_SENT_FOLDER.to_owned(),
                "trash" => DEFAULT_TRASH_FOLDER.to_owned(),
                _ => shellexpand_str(folder),
            }
        });

        Ok(alias)
    }

    /// Get the inbox folder alias.
    pub fn get_inbox_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_INBOX_FOLDER)
    }

    /// Get the drafts folder alias.
    pub fn get_drafts_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_DRAFTS_FOLDER)
    }

    /// Get the sent folder alias.
    pub fn get_sent_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_SENT_FOLDER)
    }

    /// Get the trash folder alias.
    pub fn get_trash_folder_alias(&self) -> Result<String> {
        self.get_folder_alias(DEFAULT_TRASH_FOLDER)
    }

    /// Get the folder sync strategy.
    pub fn get_folder_sync_strategy(&self) -> FolderSyncStrategy {
        self.sync
            .as_ref()
            .and_then(|c| c.strategy.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// Get the envelope listing page size if defined, otherwise
    /// return the default one.
    pub fn get_envelope_list_page_size(&self) -> usize {
        self.envelope
            .as_ref()
            .and_then(|c| c.list.as_ref())
            .and_then(|c| c.page_size)
            .unwrap_or(DEFAULT_PAGE_SIZE)
    }

    /// Get the message reading format if defined, otherwise return
    /// the default one.
    pub fn get_message_read_format(&self) -> EmailTextPlainFormat {
        self.message
            .as_ref()
            .and_then(|c| c.read.as_ref())
            .and_then(|c| c.format.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// Get the message reading headers if defined, otherwise return
    /// the default ones.
    pub fn get_message_read_headers(&self) -> Vec<String> {
        self.message
            .as_ref()
            .and_then(|c| c.read.as_ref())
            .and_then(|c| c.headers.as_ref())
            .cloned()
            .unwrap_or_else(|| vec!["From".into(), "To".into(), "Cc".into(), "Subject".into()])
    }

    /// Get the message writing headers if defined, otherwise return
    /// the default ones.
    pub fn get_message_write_headers(&self) -> Vec<String> {
        self.message
            .as_ref()
            .and_then(|c| c.write.as_ref())
            .and_then(|c| c.headers.as_ref())
            .cloned()
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

    /// Find the message pre-send hook.
    pub fn find_message_pre_send_hook(&self) -> Option<&Cmd> {
        self.message
            .as_ref()
            .and_then(|c| c.send.as_ref())
            .and_then(|c| c.pre_hook.as_ref())
    }

    /// Return `true` if a copy of sent messages should be saved in
    /// the sent folder.
    pub fn should_save_copy_sent_message(&self) -> bool {
        self.message
            .as_ref()
            .and_then(|c| c.send.as_ref())
            .and_then(|c| c.save_copy)
            .unwrap_or_default()
    }

    /// Generate a template interpreter with prefilled options from
    /// the current user account configuration.
    pub fn generate_tpl_interpreter(&self) -> MimeInterpreterBuilder {
        let builder =
            MimeInterpreterBuilder::new().with_save_attachments_dir(self.get_downloads_dir());

        #[cfg(feature = "pgp")]
        if let Some(ref pgp) = self.pgp {
            return builder.with_pgp(pgp.clone());
        }

        builder
    }

    /// Get the envelope listing datetime format, otherwise return the
    /// default one.
    pub fn get_envelope_list_datetime_fmt(&self) -> String {
        self.envelope
            .as_ref()
            .and_then(|c| c.list.as_ref())
            .and_then(|c| c.datetime_fmt.clone())
            .unwrap_or_else(|| String::from("%F %R%:z"))
    }

    /// Return `true` if the envelope listing datetime local timezone
    /// option is enabled.
    pub fn has_envelope_list_datetime_local_tz(&self) -> bool {
        self.envelope
            .as_ref()
            .and_then(|c| c.list.as_ref())
            .and_then(|c| c.datetime_local_tz)
            .unwrap_or_default()
    }
}

impl<'a> Into<Address<'a>> for AccountConfig {
    fn into(self) -> Address<'a> {
        Address::Address(EmailAddress {
            name: self.display_name.map(Into::into),
            email: self.email.into(),
        })
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
