mod pgp;

pub use self::pgp::{PgpDecrypt, PgpEncrypt, PgpSign, PgpVerify};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Encrypt {
    #[default]
    None,
    Pgp(PgpEncrypt),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Decrypt {
    #[default]
    None,
    Pgp(PgpDecrypt),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Sign {
    #[default]
    None,
    Pgp(PgpSign),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Verify {
    #[default]
    None,
    Pgp(PgpVerify),
}
