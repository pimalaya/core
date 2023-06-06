//! Account config module.
//!
//! This module contains the representation of the user's current
//! account configuration.

use dirs::data_dir;
use log::warn;
use mail_builder::headers::address::{Address, EmailAddress};
use pimalaya_email_tpl::TplInterpreter;
use pimalaya_oauth2::{AuthorizationCodeGrant, Client, RefreshAccessToken};
use pimalaya_process::Cmd;
use pimalaya_secret::Secret;
use shellexpand;
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs, io,
    ops::{Deref, DerefMut},
    path::PathBuf,
    result, vec,
};
use thiserror::Error;

use crate::{
    folder::sync::Strategy as SyncFoldersStrategy, BackendConfig, EmailHooks, EmailTextPlainFormat,
    SenderConfig,
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
    EncryptFileError(#[source] pimalaya_process::Error),
    #[error("cannot decrypt file using pgp")]
    DecryptFileError(#[source] pimalaya_process::Error),
    #[error("cannot parse download file name from {0}")]
    ParseDownloadFileNameError(PathBuf),

    #[error("cannot get sync directory from XDG_DATA_HOME")]
    GetXdgDataDirError,
    #[error("cannot create sync directories")]
    CreateXdgDataDirsError(#[source] io::Error),

    #[error("cannot configure imap oauth2")]
    OAuth2AuthorizationCodeGrantError(#[from] pimalaya_oauth2::Error),

    #[error("cannot get imap oauth2 access token from global keyring")]
    GetOAuth2AccessTokenError(#[source] pimalaya_secret::Error),
    #[error("cannot set imap oauth2 access token")]
    SetOAuth2AccessTokenError(#[source] pimalaya_secret::Error),
    #[error("cannot delete imap oauth2 access token from global keyring")]
    DeleteOAuth2AccessTokenError(#[source] pimalaya_secret::Error),

    #[error("cannot get imap oauth2 refresh token")]
    GetOAuth2RefreshTokenError(#[source] pimalaya_secret::Error),
    #[error("cannot set imap oauth2 refresh token")]
    SetOAuth2RefreshTokenError(#[source] pimalaya_secret::Error),
    #[error("cannot delete imap oauth2 refresh token from global keyring")]
    DeleteOAuth2RefreshTokenError(#[source] pimalaya_secret::Error),

    #[error("cannot get imap oauth2 client secret from user")]
    GetOAuth2ClientSecretFromUserError(#[source] io::Error),
    #[error("cannot get imap oauth2 client secret from global keyring")]
    GetOAuth2ClientSecretFromKeyring(#[source] pimalaya_secret::Error),
    #[error("cannot save imap oauth2 client secret into global keyring")]
    SetOAuth2ClientSecretIntoKeyringError(#[source] pimalaya_secret::Error),
    #[error("cannot delete imap oauth2 client secret from global keyring")]
    DeleteOAuth2ClientSecretError(#[source] pimalaya_secret::Error),

    #[error("cannot get imap password from user")]
    GetPasswdFromUserError(#[source] io::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdFromKeyring(#[source] pimalaya_secret::Error),
    #[error("cannot save imap password into global keyring")]
    SetPasswdIntoKeyringError(#[source] pimalaya_secret::Error),
    #[error("cannot delete imap password from global keyring")]
    DeletePasswdError(#[source] pimalaya_secret::Error),

    #[error("cannot refresh imap oauth2 access token")]
    RefreshOAuth2AccessTokenError(#[source] pimalaya_oauth2::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the configuration of the user account.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub email_reading_verify_cmd: Option<Cmd>,
    /// Represents the command used to decrypt an email.
    pub email_reading_decrypt_cmd: Option<Cmd>,
    /// Represents the command used to sign an email.
    pub email_writing_sign_cmd: Option<Cmd>,
    /// Represents the command used to encrypt an email.
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
    ///     ..AccountConfig::default()
    /// };
    /// ```
    pub email_sending_save_copy: bool,
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

    pub backend: BackendConfig,
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
            email_reading_headers: Default::default(),
            email_reading_format: Default::default(),
            email_reading_verify_cmd: Default::default(),
            email_reading_decrypt_cmd: Default::default(),
            email_writing_sign_cmd: Default::default(),
            email_writing_encrypt_cmd: Default::default(),
            email_writing_headers: Default::default(),
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

    pub fn generate_tpl_interpreter(&self) -> TplInterpreter {
        TplInterpreter::new()
            .some_pgp_decrypt_cmd(self.email_reading_decrypt_cmd.clone())
            .some_pgp_verify_cmd(self.email_reading_verify_cmd.clone())
            .save_attachments_dir(self.downloads_dir())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PasswdConfig {
    pub passwd: Secret,
}

impl Deref for PasswdConfig {
    type Target = Secret;

    fn deref(&self) -> &Self::Target {
        &self.passwd
    }
}

impl DerefMut for PasswdConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.passwd
    }
}

impl PasswdConfig {
    pub fn reset(&self) -> Result<()> {
        self.delete_keyring_entry_secret()
            .map_err(Error::DeletePasswdError)?;
        Ok(())
    }

    pub fn configure(&self, get_passwd: impl Fn() -> io::Result<String>) -> Result<()> {
        match self.find() {
            Ok(None) => {
                warn!("cannot find imap password from keyring, setting it");
                let passwd = get_passwd().map_err(Error::GetPasswdFromUserError)?;
                self.set_keyring_entry_secret(passwd)
                    .map_err(Error::SetPasswdIntoKeyringError)?;
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(err) => Err(Error::GetPasswdFromKeyring(err)),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OAuth2Config {
    pub method: OAuth2Method,
    pub client_id: String,
    pub client_secret: Secret,
    pub auth_url: String,
    pub token_url: String,
    pub access_token: Secret,
    pub refresh_token: Secret,
    pub pkce: bool,
    pub scopes: OAuth2Scopes,
    pub redirect_host: String,
    pub redirect_port: u16,
}

impl OAuth2Config {
    pub fn reset(&self) -> Result<()> {
        self.client_secret
            .delete_keyring_entry_secret()
            .map_err(Error::DeleteOAuth2ClientSecretError)?;
        self.access_token
            .delete_keyring_entry_secret()
            .map_err(Error::DeleteOAuth2AccessTokenError)?;
        self.refresh_token
            .delete_keyring_entry_secret()
            .map_err(Error::DeleteOAuth2RefreshTokenError)?;
        Ok(())
    }

    pub fn configure(&self, get_client_secret: impl Fn() -> io::Result<String>) -> Result<()> {
        if self.access_token.get().is_ok() {
            return Ok(());
        }

        let set_client_secret = || {
            self.client_secret
                .set_keyring_entry_secret(
                    get_client_secret().map_err(Error::GetOAuth2ClientSecretFromUserError)?,
                )
                .map_err(Error::SetOAuth2ClientSecretIntoKeyringError)
        };

        let client_secret = match self.client_secret.find() {
            Ok(None) => {
                warn!("cannot find imap oauth2 client secret from keyring, setting it");
                set_client_secret()
            }
            Ok(Some(client_secret)) => Ok(client_secret),
            Err(err) => Err(Error::GetOAuth2ClientSecretFromKeyring(err)),
        }?;

        let client = Client::new(
            self.client_id.clone(),
            client_secret,
            self.auth_url.clone(),
            self.token_url.clone(),
        )?
        .with_redirect_host(self.redirect_host.clone())
        .with_redirect_port(self.redirect_port)
        .build()?;

        let mut auth_code_grant = AuthorizationCodeGrant::new()
            .with_redirect_host(self.redirect_host.clone())
            .with_redirect_port(self.redirect_port);

        if self.pkce {
            auth_code_grant = auth_code_grant.with_pkce();
        }

        for scope in self.scopes.clone() {
            auth_code_grant = auth_code_grant.with_scope(scope);
        }

        let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

        println!("To enable OAuth2, click on the following link:");
        println!("");
        println!("{}", redirect_url.to_string());

        let (access_token, refresh_token) =
            auth_code_grant.wait_for_redirection(&client, csrf_token)?;

        self.access_token
            .set_keyring_entry_secret(access_token)
            .map_err(Error::SetOAuth2AccessTokenError)?;

        if let Some(refresh_token) = &refresh_token {
            self.refresh_token
                .set_keyring_entry_secret(refresh_token)
                .map_err(Error::SetOAuth2RefreshTokenError)?;
        }

        Ok(())
    }

    pub fn refresh_access_token(&self) -> Result<String> {
        let client_secret = self
            .client_secret
            .get()
            .map_err(Error::GetOAuth2ClientSecretFromKeyring)?;

        let client = Client::new(
            self.client_id.clone(),
            client_secret,
            self.auth_url.clone(),
            self.token_url.clone(),
        )?
        .with_redirect_host(self.redirect_host.clone())
        .with_redirect_port(self.redirect_port)
        .build()?;

        let refresh_token = self
            .refresh_token
            .get()
            .map_err(Error::GetOAuth2RefreshTokenError)?;

        let (access_token, refresh_token) = RefreshAccessToken::new()
            .refresh_access_token(&client, refresh_token)
            .map_err(Error::RefreshOAuth2AccessTokenError)?;

        self.access_token
            .set_keyring_entry_secret(&access_token)
            .map_err(Error::SetOAuth2AccessTokenError)?;

        if let Some(refresh_token) = &refresh_token {
            self.refresh_token
                .set_keyring_entry_secret(refresh_token)
                .map_err(Error::SetOAuth2RefreshTokenError)?;
        }

        Ok(access_token)
    }

    pub fn access_token(&self) -> Result<String> {
        self.access_token
            .get()
            .map_err(Error::GetOAuth2AccessTokenError)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum OAuth2Method {
    #[default]
    XOAuth2,
    OAuthBearer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OAuth2Scopes {
    Scope(String),
    Scopes(Vec<String>),
}

impl Default for OAuth2Scopes {
    fn default() -> Self {
        Self::Scopes(Vec::new())
    }
}

impl IntoIterator for OAuth2Scopes {
    type Item = String;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Scope(scope) => vec![scope].into_iter(),
            Self::Scopes(scopes) => scopes.into_iter(),
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
