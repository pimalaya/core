//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.
#[cfg(feature = "pgp-commands")]
pub mod cmds;
#[cfg(feature = "pgp-gpg")]
pub mod gpg;
#[cfg(feature = "pgp-native")]
pub mod native;

use mml::pgp::Pgp;
use serde::{Deserialize, Serialize};
use std::io;

use crate::Result;

#[cfg(feature = "pgp-commands")]
#[doc(inline)]
pub use self::cmds::CmdsPgpConfig;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use self::gpg::GpgConfig;
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use self::native::NativePgpConfig;

/// The PGP configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "backend")]
pub enum PgpConfig {
    #[cfg(feature = "pgp-commands")]
    #[serde(aliases = ["cmd", "command", "commands"])]
    /// Commands configuration.
    Cmds(CmdsPgpConfig),

    #[cfg(feature = "pgp-gpg")]
    /// GPG configuration.
    Gpg(GpgConfig),

    #[cfg(feature = "pgp-native")]
    /// Native configuration.
    Native(NativePgpConfig),
}

impl Into<Pgp> for PgpConfig {
    fn into(self) -> Pgp {
        match self {
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(config) => config.into(),
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(config) => config.into(),
            #[cfg(feature = "pgp-native")]
            Self::Native(config) => config.into(),
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
