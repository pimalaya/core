#[cfg(feature = "commands")]
pub mod cmds;
#[cfg(feature = "gpg")]
pub mod gpg;
#[cfg(feature = "native")]
pub mod native;
pub mod wkd;

#[cfg(feature = "commands")]
pub use self::cmds::PgpCmds;
#[cfg(feature = "gpg")]
pub use self::gpg::PgpGpg;
#[cfg(feature = "native")]
pub use self::native::PgpNative;

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "commands")]
    #[error(transparent)]
    CmdsError(#[from] cmds::Error),
    #[cfg(feature = "gpg")]
    #[error(transparent)]
    GpgError(#[from] gpg::Error),
    #[cfg(feature = "native")]
    #[error(transparent)]
    NativeError(#[from] native::Error),

    #[error(transparent)]
    WkdError(#[from] wkd::Error),

    #[error("cannot perform pgp action: pgp not configured")]
    None,
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The PGP abstraction.
///
/// PGP actions can be performed using various backends.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Pgp {
    /// Does not perform PGP actions.
    #[default]
    None,

    /// Uses shell commands to perform PGP actions.
    #[cfg(feature = "commands")]
    Cmds(PgpCmds),

    /// Uses [`gpgme`] to perform PGP actions.
    #[cfg(feature = "gpg")]
    Gpg(PgpGpg),

    /// Uses the native PGP implementation [`pgp`] to perform PGP
    /// actions.
    #[cfg(feature = "native")]
    Native(PgpNative),
}

impl Pgp {
    pub fn configure(&self) {
        match self {
            Self::None => (),
            #[cfg(feature = "commands")]
            Self::Cmds(_cmds) => (),
            #[cfg(feature = "gpg")]
            Self::Gpg(_gpg) => (),
            #[cfg(feature = "native")]
            Self::Native(_native) => (),
        }
    }

    pub async fn encrypt(
        &self,
        _data: &[u8],
        _recipients: impl Iterator<Item = impl AsRef<str>>,
    ) -> Result<Vec<u8>> {
        unimplemented!()
    }

    pub async fn decrypt(&self, _data: &[u8], _receiver: impl AsRef<str>) -> Result<Vec<u8>> {
        unimplemented!()
    }

    pub async fn sign(&self, data: &[u8], sender: impl ToString) -> Result<Vec<u8>> {
        match self {
            Self::None => Err(Error::None),
            #[cfg(feature = "commands")]
            Self::Cmds(cmds) => cmds.sign(data, sender).await,
            #[cfg(feature = "gpg")]
            Self::Gpg(gpg) => gpg.sign(data, sender),
            #[cfg(feature = "native")]
            Self::Native(native) => native.sign(data, sender),
        }
    }

    pub async fn verify(
        &self,
        _data: &[u8],
        _recipients: impl Iterator<Item = impl AsRef<str>>,
    ) -> Result<Vec<u8>> {
        unimplemented!()
    }
}
