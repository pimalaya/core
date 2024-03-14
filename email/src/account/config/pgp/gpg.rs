use mml::pgp::{Gpg, Pgp};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct GpgConfig;

impl From<GpgConfig> for Pgp {
    fn from(_val: GpgConfig) -> Self {
        // TODO: retrieve Gpg home_dir from configurations.
        Pgp::Gpg(Gpg { home_dir: None })
    }
}
