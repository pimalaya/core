//! # Output
//!
//! Module dedicated to command output. It only exposes an [`Output`]
//! struct, a wrapper around raw `Vec<u8>` output.

use std::ops::{Deref, DerefMut};

use crate::{Error, Result};

/// Wrapper around command output.
///
/// The only role of this struct is to provide convenient functions to
/// export command output.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Output(Vec<u8>);

impl Output {
    pub fn new(output: impl IntoIterator<Item = u8>) -> Self {
        Self::from(output.into_iter().collect::<Vec<_>>())
    }

    /// Reads the command output as string lossy.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self).to_string()
    }
}

impl Deref for Output {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Output {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for Output {
    fn from(output: Vec<u8>) -> Self {
        Self(output)
    }
}

impl From<Output> for Vec<u8> {
    fn from(output: Output) -> Self {
        output.0
    }
}

impl TryFrom<Output> for String {
    type Error = Error;

    fn try_from(output: Output) -> Result<Self> {
        String::from_utf8(output.into()).map_err(Error::ParseOutputAsUtf8StringError)
    }
}
