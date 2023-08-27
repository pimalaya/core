use mml::{Gpg, Pgp};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GpgConfig;

impl Into<Pgp> for GpgConfig {
    fn into(self) -> Pgp {
        Pgp::Gpg(Gpg { home_dir: None })
    }
}
