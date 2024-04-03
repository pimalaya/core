#[cfg(feature = "account-discovery")]
use hyper::{StatusCode, Uri};
use std::{io, path::PathBuf};

/// Errors related to account configuration.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot parse download file name from {0}")]
    ParseDownloadFileNameError(PathBuf),
    #[error("cannot get sync directory from XDG_DATA_HOME")]
    GetXdgDataDirSyncError,
    #[error("cannot create sync directories")]
    CreateXdgDataDirsSyncError(#[source] io::Error),
    #[error("cannot get file name from path {0}")]
    GetFileNameFromPathSyncError(PathBuf),
    #[error("cannot create oauth2 client")]
    InitOauthClientError(#[source] oauth::Error),
    #[error("cannot create oauth2 client")]
    BuildOauthClientError(#[source] oauth::Error),
    #[error("cannot wait for oauth2 redirection error")]
    WaitForOauthRedirectionError(#[source] oauth::Error),

    #[error("cannot get oauth2 access token from global keyring")]
    GetAccessTokenOauthError(#[source] secret::Error),
    #[error("cannot set oauth2 access token")]
    SetAccessTokenOauthError(#[source] secret::Error),
    #[error("cannot refresh oauth2 access token")]
    RefreshAccessTokenOauthError(#[source] oauth::Error),
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
    #[cfg(feature = "account-discovery")]
    #[error("cannot do txt lookup: {0}")]
    TXTLookUpFailure(#[source] hickory_resolver::error::ResolveError),
    #[cfg(feature = "account-discovery")]
    #[error("cannot do mx lookup: {0}")]
    MXLookUpFailure(#[source] hickory_resolver::error::ResolveError),
    #[cfg(feature = "account-discovery")]
    #[error("cannot do srv lookup: {0}")]
    SRVLookUpFailure(#[source] hickory_resolver::error::ResolveError),
    #[cfg(feature = "account-discovery")]
    #[error("cannot get autoconfig from {0}: {1}")]
    GetAutoConfigError(Uri, StatusCode),
    #[cfg(feature = "account-discovery")]
    #[error("cannot do a get request for autoconfig from {0}: {1}")]
    GetConnectionAutoConfigError(Uri, #[source] hyper::Error),
    #[cfg(feature = "account-discovery")]
    #[error("cannot get the body of response for autoconfig from {0}: {1}")]
    ToBytesAutoConfigError(Uri, #[source] hyper::Error),
    #[cfg(feature = "account-discovery")]
    #[error("cannot decode the body of response for autoconfig from {0}: {1}")]
    SerdeXmlFailedForAutoConfig(Uri, #[source] serde_xml_rs::Error),
    #[cfg(feature = "account-discovery")]
    #[error("cannot parse email {0}: {1}")]
    ParsingEmailAddress(String, #[source] email_address::Error),
}

impl crate::EmailError for Error {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl From<Error> for Box<dyn crate::EmailError> {
    fn from(value: Error) -> Self {
        Box::new(value)
    }
}
