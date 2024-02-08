use mml::pgp::{Gpg, Pgp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GpgConfig;

impl From<GpgConfig> for Pgp {
    fn from(val: GpgConfig) -> Self {
        Pgp::Gpg(Gpg { home_dir: None })
    }
}
