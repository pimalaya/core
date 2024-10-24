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
pub use self::cmds::CmdsPgpConfig;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use self::gpg::GpgConfig;
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use self::native::NativePgpConfig;
#[doc(inline)]
pub use super::{Error, Result};

/// The PGP configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case", tag = "backend"),
    serde(from = "derive::PgpConfig")
)]
pub enum PgpConfig {
    #[cfg(feature = "pgp-commands")]
    #[cfg_attr(feature = "derive", serde(rename = "commands"))]
    /// Commands configuration.
    Cmds(CmdsPgpConfig),

    #[cfg(feature = "pgp-gpg")]
    /// GPG configuration.
    Gpg(GpgConfig),

    #[cfg(feature = "pgp-native")]
    /// Native configuration.
    Native(NativePgpConfig),
}

impl From<PgpConfig> for Pgp {
    fn from(val: PgpConfig) -> Self {
        match val {
            #[cfg(feature = "pgp-commands")]
            PgpConfig::Cmds(config) => config.into(),
            #[cfg(feature = "pgp-gpg")]
            PgpConfig::Gpg(config) => config.into(),
            #[cfg(feature = "pgp-native")]
            PgpConfig::Native(config) => config.into(),
        }
    }
}

impl PgpConfig {
    pub async fn reset(&self) -> Result<()> {
        match self {
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(..) => Ok(()),
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
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(..) => Ok(()),
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(..) => Ok(()),
            #[cfg(feature = "pgp-native")]
            Self::Native(config) => config.configure(email, passwd).await,
        }
    }
}
