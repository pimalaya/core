use mml::pgp::{Gpg, Pgp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GpgConfig;

impl From<GpgConfig> for Pgp {
    fn from(_val: GpgConfig) -> Self {
        // TODO: retrieve Gpg home_dir from configurations.
        Pgp::Gpg(Gpg { home_dir: None })
    }
}
