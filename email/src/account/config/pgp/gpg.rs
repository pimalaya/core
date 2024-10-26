use mml::pgp::{Pgp, PgpGpg};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct PgpGpgConfig;

impl From<PgpGpgConfig> for Pgp {
    fn from(_config: PgpGpgConfig) -> Self {
        // TODO: retrieve Gpg home_dir from configurations.
        Pgp::Gpg(PgpGpg { home_dir: None })
    }
}
