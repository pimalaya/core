use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "backend")]
pub enum PgpConfig {
    #[cfg(feature = "pgp-commands")]
    #[serde(alias = "cmd", alias = "cmds", alias = "command")]
    Commands(super::CmdsPgpConfig),
    #[cfg(not(feature = "pgp-commands"))]
    #[serde(alias = "cmd", alias = "cmds", alias = "command")]
    #[serde(skip_serializing, deserialize_with = "missing_commands_feature")]
    Commands,

    #[cfg(feature = "pgp-gpg")]
    Gpg(super::GpgConfig),
    #[cfg(not(feature = "pgp-gpg"))]
    #[serde(skip_serializing, deserialize_with = "missing_gpg_feature")]
    Gpg,

    #[cfg(feature = "pgp-native")]
    Native(super::NativePgpConfig),
    #[cfg(not(feature = "pgp-native"))]
    #[serde(alias = "cmd", alias = "cmds", alias = "command")]
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
            PgpConfig::Commands(config) => super::PgpConfig::Cmds(config),
            #[cfg(not(feature = "pgp-commands"))]
            PgpConfig::Commands => unreachable!(),

            #[cfg(feature = "pgp-gpg")]
            PgpConfig::Gpg(config) => super::PgpConfig::Gpg(config),
            #[cfg(not(feature = "pgp-gpg"))]
            PgpConfig::Gpg => unreachable!(),

            #[cfg(feature = "pgp-native")]
            PgpConfig::Native(config) => super::PgpConfig::Native(config),
            #[cfg(not(feature = "pgp-native"))]
            PgpConfig::Native => unreachable!(),
        }
    }
}
