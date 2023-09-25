//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.
#[cfg(feature = "pgp-commands")]
pub mod cmds;
#[cfg(feature = "pgp-gpg")]
pub mod gpg;
#[cfg(feature = "pgp-native")]
pub mod native;

#[cfg(feature = "pgp-commands")]
#[doc(inline)]
pub use mml::pgp::CmdsPgp;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use mml::pgp::Gpg;
#[cfg(feature = "pgp")]
#[doc(inline)]
pub use mml::pgp::Pgp;
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use mml::pgp::{
    NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, SignedPublicKey, SignedSecretKey,
};
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
// TODO: `Gpg` variant using `libgpgme`
// TODO: `Autocrypt` variant based on `pimalaya-pgp`
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpConfig {
    /// No configuration.
    #[default]
    None,

    #[cfg(feature = "pgp-commands")]
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
            Self::None => Pgp::None,
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
            Self::None => Ok(()),
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
            Self::None => Ok(()),
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(..) => Ok(()),
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(..) => Ok(()),
            #[cfg(feature = "pgp-native")]
            Self::Native(config) => config.configure(email, passwd).await,
        }
    }
}
