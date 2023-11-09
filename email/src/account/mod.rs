//! Module dedicated to account management.
//!
//! This module contains everything related to account configuration,
//! plus everything you need to synchronize a remote account using a
//! local Maildir backend. It also contains common code related to
//! PGP.

pub mod config;
pub mod sync;

#[cfg(feature = "pgp-commands")]
#[doc(inline)]
pub use self::config::CmdsPgpConfig;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use self::config::GpgConfig;
#[cfg(feature = "pgp")]
#[doc(inline)]
pub use self::config::PgpConfig;
#[doc(inline)]
pub use self::config::{
    AccountConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig, DEFAULT_DRAFTS_FOLDER,
    DEFAULT_INBOX_FOLDER, DEFAULT_PAGE_SIZE, DEFAULT_SENT_FOLDER, DEFAULT_SIGNATURE_DELIM,
    DEFAULT_TRASH_FOLDER,
};
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use self::config::{NativePgpConfig, NativePgpSecretKey, SignedPublicKey, SignedSecretKey};

pub trait WithAccountConfig: Send + Sync {
    fn account_config(&self) -> &AccountConfig;
}
