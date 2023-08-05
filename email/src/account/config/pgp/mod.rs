//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.
#[cfg(feature = "cmds-pgp")]
pub mod cmds;
#[cfg(feature = "gpg")]
pub mod gpg;
#[cfg(feature = "native-pgp")]
pub mod native;

#[cfg(feature = "cmds-pgp")]
#[doc(inline)]
pub use pimalaya_email_tpl::CmdsPgp;
#[cfg(feature = "gpg")]
#[doc(inline)]
pub use pimalaya_email_tpl::Gpg;
use pimalaya_email_tpl::Pgp;
#[cfg(feature = "native-pgp")]
#[doc(inline)]
pub use pimalaya_email_tpl::{
    NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, SignedPublicKey, SignedSecretKey,
};
use std::io;

use crate::Result;

#[cfg(feature = "cmds-pgp")]
#[doc(inline)]
pub use self::cmds::CmdsPgpConfig;
#[cfg(feature = "gpg")]
#[doc(inline)]
pub use self::gpg::GpgConfig;
#[cfg(feature = "native-pgp")]
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

    #[cfg(feature = "cmds-pgp")]
    /// Commands configuration.
    Cmds(CmdsPgpConfig),

    #[cfg(feature = "gpg")]
    /// GPG configuration.
    Gpg(GpgConfig),

    #[cfg(feature = "native-pgp")]
    /// Native configuration.
    Native(NativePgpConfig),
}

impl Into<Pgp> for PgpConfig {
    fn into(self) -> Pgp {
        match self {
            Self::None => Pgp::None,
            #[cfg(feature = "cmds-pgp")]
            Self::Cmds(config) => config.into(),
            #[cfg(feature = "gpg")]
            Self::Gpg(config) => config.into(),
            #[cfg(feature = "native-pgp")]
            Self::Native(config) => config.into(),
        }
    }
}

impl PgpConfig {
    pub async fn reset(&self) -> Result<()> {
        match self {
            Self::None => Ok(()),
            #[cfg(feature = "cmds-pgp")]
            Self::Cmds(..) => Ok(()),
            #[cfg(feature = "gpg")]
            Self::Gpg(..) => Ok(()),
            #[cfg(feature = "native-pgp")]
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
            #[cfg(feature = "cmds-pgp")]
            Self::Cmds(..) => Ok(()),
            #[cfg(feature = "gpg")]
            Self::Gpg(..) => Ok(()),
            #[cfg(feature = "native-pgp")]
            Self::Native(config) => config.configure(email, passwd).await,
        }
    }
}
