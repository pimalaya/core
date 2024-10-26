use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum PgpConfig {
    #[cfg(feature = "pgp-commands")]
    Commands(super::PgpCommandsConfig),
    #[cfg(not(feature = "pgp-commands"))]
    #[serde(skip_serializing, deserialize_with = "missing_commands_feature")]
    Commands,

    #[cfg(feature = "pgp-gpg")]
    Gpg(super::PgpGpgConfig),
    #[cfg(not(feature = "pgp-gpg"))]
    #[serde(skip_serializing, deserialize_with = "missing_gpg_feature")]
    Gpg,

    #[cfg(feature = "pgp-native")]
    Native(super::PgpNativeConfig),
    #[cfg(not(feature = "pgp-native"))]
    #[serde(skip_serializing, deserialize_with = "missing_native_feature")]
    Native,
}

#[cfg(not(feature = "pgp-commands"))]
fn missing_commands_feature<'de, D: serde::Deserializer<'de>>(_: D) -> Result<(), D::Error> {
    Err(serde::de::Error::custom(
        "missing `pgp-commands` cargo feature",
    ))
}

#[cfg(not(feature = "pgp-gpg"))]
fn missing_gpg_feature<'de, D: serde::Deserializer<'de>>(_: D) -> Result<(), D::Error> {
    Err(serde::de::Error::custom("missing `pgp-gpg` cargo feature"))
}

#[cfg(not(feature = "pgp-native"))]
fn missing_native_feature<'de, D: serde::Deserializer<'de>>(_: D) -> Result<(), D::Error> {
    Err(serde::de::Error::custom(
        "missing `pgp-native` cargo feature",
    ))
}

impl From<PgpConfig> for super::PgpConfig {
    fn from(config: PgpConfig) -> Self {
        match config {
            #[cfg(feature = "pgp-commands")]
            PgpConfig::Commands(config) => super::PgpConfig::Commands(config),
            #[cfg(not(feature = "pgp-commands"))]
            PgpConfig::Commands => super::PgpConfig::None,

            #[cfg(feature = "pgp-gpg")]
            PgpConfig::Gpg(config) => super::PgpConfig::Gpg(config),
            #[cfg(not(feature = "pgp-gpg"))]
            PgpConfig::Gpg => super::PgpConfig::None,

            #[cfg(feature = "pgp-native")]
            PgpConfig::Native(config) => super::PgpConfig::Native(config),
            #[cfg(not(feature = "pgp-native"))]
            PgpConfig::Native => super::PgpConfig::None,
        }
    }
}
