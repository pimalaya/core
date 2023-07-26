use pimalaya_pgp::{SignedPublicKey, SignedSecretKey};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgpEncrypt {
    pub pkeys: Vec<SignedPublicKey>,
}

impl PgpEncrypt {
    pub fn new(pkeys: impl IntoIterator<Item = SignedPublicKey>) -> Self {
        Self {
            pkeys: pkeys.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgpDecrypt {
    pub skey: SignedSecretKey,
}

impl PgpDecrypt {
    pub fn new(skey: SignedSecretKey) -> Self {
        Self { skey }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgpSign {
    pub skey: SignedSecretKey,
}

impl PgpSign {
    pub fn new(skey: SignedSecretKey) -> Self {
        Self { skey }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgpVerify {
    pub pkey: SignedPublicKey,
}

impl PgpVerify {
    pub fn new(pkey: SignedPublicKey) -> Self {
        Self { pkey }
    }
}
