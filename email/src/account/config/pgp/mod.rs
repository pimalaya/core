//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.
#[cfg(feature = "pgp-commands")]
pub mod cmds;
#[cfg(feature = "derive")]
pub mod derive;
#[cfg(feature = "pgp-gpg")]
pub mod gpg;
#[cfg(feature = "pgp-native")]
pub mod native;

use std::io;

use mml::pgp::Pgp;

#[cfg(feature = "pgp-commands")]
#[doc(inline)]
pub use self::cmds::PgpCommandsConfig;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use self::gpg::PgpGpgConfig;
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use self::native::PgpNativeConfig;
#[doc(inline)]
pub use super::{Error, Result};

/// The PGP configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case", tag = "type"),
    serde(from = "derive::PgpConfig")
)]
pub enum PgpConfig {
    #[default]
    None,
    /// Commands configuration.
    #[cfg(feature = "pgp-commands")]
    Commands(PgpCommandsConfig),
    /// GPG configuration.
    #[cfg(feature = "pgp-gpg")]
    Gpg(PgpGpgConfig),
    /// Native configuration.
    #[cfg(feature = "pgp-native")]
    Native(PgpNativeConfig),
}

impl From<PgpConfig> for Pgp {
    fn from(config: PgpConfig) -> Self {
        match config {
            PgpConfig::None => Pgp::None,
            #[cfg(feature = "pgp-commands")]
            PgpConfig::Commands(config) => Pgp::from(config),
            #[cfg(feature = "pgp-gpg")]
            PgpConfig::Gpg(config) => Pgp::from(config),
            #[cfg(feature = "pgp-native")]
            PgpConfig::Native(config) => Pgp::from(config),
        }
    }
}

impl PgpConfig {
    pub async fn reset(&self) -> Result<()> {
        match self {
            Self::None => Ok(()),
            #[cfg(feature = "pgp-commands")]
            Self::Commands(..) => Ok(()),
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(..) => Ok(()),
            #[cfg(feature = "pgp-native")]
            Self::Native(config) => config.reset().await,
        }
    }

    #[allow(unused)]
    pub async fn configure(
        &self,
        email: impl ToString,
        passwd: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        match self {
            Self::None => Ok(()),
            #[cfg(feature = "pgp-commands")]
            Self::Commands(..) => Ok(()),
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(..) => Ok(()),
            #[cfg(feature = "pgp-native")]
            Self::Native(config) => config.configure(email, passwd).await,
        }
    }
}
