#[cfg(feature = "keyring")]
use keyring::KeyringEntry;
#[cfg(feature = "command")]
use process::Command;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Secret {
    Raw(String),

    #[cfg(feature = "command")]
    #[serde(alias = "cmd")]
    Command(Command),
    #[cfg(not(feature = "command"))]
    #[serde(alias = "cmd")]
    #[serde(skip_serializing, deserialize_with = "missing_command_feature")]
    Command,

    #[cfg(feature = "keyring")]
    #[serde(rename = "keyring")]
    KeyringEntry(KeyringEntry),
    #[cfg(not(feature = "keyring"))]
    #[serde(alias = "keyring")]
    #[serde(skip_serializing, deserialize_with = "missing_keyring_feature")]
    KeyringEntry,

    /// The secret is not defined.
    #[default]
    #[serde(skip_serializing)]
    Undefined,
}

impl From<Secret> for crate::Secret {
    fn from(secret: Secret) -> Self {
        match secret {
            Secret::Raw(secret) => Self::Raw(secret),
            #[cfg(feature = "command")]
            Secret::Command(cmd) => Self::Command(cmd),
            #[cfg(not(feature = "command"))]
            #[serde(alias = "cmd")]
            Command => Self::Undefined,
            #[cfg(feature = "keyring")]
            Secret::KeyringEntry(entry) => Self::KeyringEntry(entry),
            #[cfg(not(feature = "keyring"))]
            Secret::KeyringEntry => Self::Undefined,
            Secret::Undefined => Self::Undefined,
        }
    }
}

#[cfg(not(feature = "command"))]
fn missing_command_feature<'de, D: serde::Deserializer<'de>>(_: D) -> Result<(), D::Error> {
    Err(serde::de::Error::custom("missing `command` cargo feature"))
}

#[cfg(not(feature = "keyring"))]
fn missing_keyring_feature<'de, D: serde::Deserializer<'de>>(_: D) -> Result<(), D::Error> {
    Err(serde::de::Error::custom("missing `keyring` cargo feature"))
}
