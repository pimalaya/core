//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.

use pimalaya_process::Cmd;
use thiserror::Error;

/// Errors related to PGP configuration.
#[derive(Debug, Error)]
pub enum Error {
    //
}

/// The PGP configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpConfig {
    #[default]
    None,

    /// Native configuration.
    Native(PgpNativeConfig),

    /// GPG configuration.
    Gpg(PgpGpgConfig),

    /// Commands configuration.
    Cmd(PgpCmdConfig),
}

/// The native PGP configuration.
///
/// This configuration is based on the [`pgp`] crate, which provides a
/// native Rust implementation of the PGP standard.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpNativeConfig {
    //
}

/// The GPG configuration.
///
/// This configuration is based on the [`gpgme`] crate, which provides
/// bindings to the libgpgme.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpGpgConfig {
    //
}

/// The PGP commands configuration.
///
/// This configuration is based on system commands.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpCmdConfig {
    encrypt_cmd: Cmd,
    decrypt_cmd: Cmd,
    sign_cmd: Cmd,
    verify_cmd: Cmd,
}
