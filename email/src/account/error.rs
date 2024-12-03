use std::{io, path::PathBuf, result};

#[cfg(feature = "autoconfig")]
use http::ureq::http::{StatusCode, Uri};
use thiserror::Error;

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get configuration of account {0}")]
    GetAccountConfigNotFoundError(String),

    #[cfg(feature = "sync")]
    #[error("cannot get sync directory from XDG_DATA_HOME")]
    GetXdgDataDirSyncError,
    #[cfg(feature = "sync")]
    #[error("cannot get invalid or missing synchronization directory {1}")]
    GetSyncDirInvalidError(#[source] shellexpand_utils::Error, PathBuf),

    #[error("cannot parse download file name from {0}")]
    ParseDownloadFileNameError(PathBuf),
    #[error("cannot get file name from path {0}")]
    GetFileNameFromPathSyncError(PathBuf),
    #[cfg(feature = "oauth2")]
    #[error("cannot create oauth2 client")]
    InitOauthClientError(#[source] oauth::v2_0::Error),
    #[cfg(feature = "oauth2")]
    #[error("cannot create oauth2 client")]
    BuildOauthClientError(#[source] oauth::v2_0::Error),
    #[cfg(feature = "oauth2")]
    #[error("cannot wait for oauth2 redirection error")]
    WaitForOauthRedirectionError(#[source] oauth::v2_0::Error),

    #[error("cannot get oauth2 access token from global keyring")]
    GetAccessTokenOauthError(#[source] secret::Error),
    #[error("cannot set oauth2 access token")]
    SetAccessTokenOauthError(#[source] secret::Error),
    #[cfg(feature = "oauth2")]
    #[error("cannot refresh oauth2 access token")]
    RefreshAccessTokenOauthError(#[source] oauth::v2_0::Error),
    #[error("cannot delete oauth2 access token from global keyring")]
    DeleteAccessTokenOauthError(#[source] secret::Error),

    #[error("cannot get oauth2 refresh token")]
    GetRefreshTokenOauthError(#[source] secret::Error),
    #[error("cannot set oauth2 refresh token")]
    SetRefreshTokenOauthError(#[source] secret::Error),
    #[error("cannot delete oauth2 refresh token from global keyring")]
    DeleteRefreshTokenOauthError(#[source] secret::Error),

    #[error("cannot get oauth2 client secret from user")]
    GetClientSecretFromUserOauthError(#[source] io::Error),
    #[error("cannot get oauth2 client secret from global keyring")]
    GetClientSecretFromKeyringOauthError(#[source] secret::Error),
    #[error("cannot save oauth2 client secret into global keyring")]
    SetClientSecretIntoKeyringOauthError(#[source] secret::Error),
    #[error("cannot delete oauth2 client secret from global keyring")]
    DeleteClientSecretOauthError(#[source] secret::Error),

    #[error("cannot get available port")]
    GetAvailablePortError,
    #[error("cannot get password from user")]
    GetFromUserError(#[source] io::Error),
    #[error("cannot get password from global keyring")]
    GetFromKeyringError(#[source] secret::Error),
    #[error("cannot save password into global keyring")]
    SetIntoKeyringError(#[source] secret::Error),
    #[error("cannot delete password from global keyring")]
    DeletePasswordFromKeyringError(#[source] secret::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot delete pgp key from keyring")]
    DeletePgpKeyFromKeyringError(#[source] keyring::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot delete pgp key at {1}")]
    DeletePgpKeyAtPathError(#[source] io::Error, PathBuf),
    #[cfg(feature = "pgp-native")]
    #[error("cannot generate pgp key pair for {1}")]
    GeneratePgpKeyPairError(#[source] pgp::Error, String),
    #[cfg(feature = "pgp-native")]
    #[error("cannot export secret key to armored string")]
    ExportSecretKeyToArmoredStringError(#[source] pgp::native::errors::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot export public key to armored string")]
    ExportPublicKeyToArmoredStringError(#[source] pgp::native::errors::Error),
    #[error("cannot write secret key file at {1}")]
    WriteSecretKeyFileError(#[source] io::Error, PathBuf),
    #[error("cannot write public key file at {1}")]
    WritePublicKeyFileError(#[source] io::Error, PathBuf),
    #[cfg(feature = "pgp-native")]
    #[error("cannot get public key from keyring")]
    GetPublicKeyFromKeyringError(#[source] keyring::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot set secret key to keyring")]
    SetSecretKeyToKeyringError(#[source] keyring::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot set public key to keyring")]
    SetPublicKeyToKeyringError(#[source] keyring::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot get secret key password")]
    GetPgpSecretKeyPasswdError(#[source] io::Error),
    #[cfg(feature = "pgp-native")]
    #[error("cannot create keyring entry from key: {0}")]
    KeyringError(#[from] keyring::Error),
    #[error("cannot find any MX record at {0}")]
    GetMxRecordNotFoundError(String),
    #[error("cannot find any mailconf TXT record at {0}")]
    GetMailconfTxtRecordNotFoundError(String),
    #[error("cannot find any SRV record at {0}")]
    GetSrvRecordNotFoundError(String),
    #[cfg(feature = "autoconfig")]
    #[error("cannot do txt lookup: {0}")]
    TXTLookUpFailure(#[source] hickory_resolver::error::ResolveError),
    #[cfg(feature = "autoconfig")]
    #[error("cannot do mx lookup: {0}")]
    MXLookUpFailure(#[source] hickory_resolver::error::ResolveError),
    #[cfg(feature = "autoconfig")]
    #[error("cannot do srv lookup: {0}")]
    SRVLookUpFailure(#[source] hickory_resolver::error::ResolveError),
    #[cfg(feature = "autoconfig")]
    #[error("cannot get autoconfig from {0}: {1}")]
    GetAutoConfigError(Uri, StatusCode),
    #[cfg(feature = "autoconfig")]
    #[error("cannot do a get request for autoconfig from {0}: {1}")]
    GetConnectionAutoConfigError(Uri, #[source] http::Error),
    #[cfg(feature = "autoconfig")]
    #[error("cannot get the body of response for autoconfig from {0}: {1}")]
    ToBytesAutoConfigError(Uri, #[source] http::Error),
    #[cfg(feature = "autoconfig")]
    #[error("cannot decode the body of response for autoconfig from {0}: {1}")]
    SerdeXmlFailedForAutoConfig(Uri, #[source] serde_xml_rs::Error),
    #[cfg(feature = "autoconfig")]
    #[error("cannot parse email {0}: {1}")]
    ParsingEmailAddress(String, #[source] email_address::Error),
}
