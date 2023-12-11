use mml::pgp::{Gpg, Pgp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GpgConfig;

impl Into<Pgp> for GpgConfig {
    fn into(self) -> Pgp {
        Pgp::Gpg(Gpg { home_dir: None })
    }
}
