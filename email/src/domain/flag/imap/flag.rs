use imap;
use std::result;
use thiserror::Error;

use crate::Flag;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse unknown imap flag {0}")]
    ParseFlagError(String),
}

pub type Result<T> = result::Result<T, Error>;

impl Flag {
    pub fn to_imap_query(&self) -> String {
        match self {
            Flag::Seen => String::from("\\Seen"),
            Flag::Answered => String::from("\\Answered"),
            Flag::Flagged => String::from("\\Flagged"),
            Flag::Deleted => String::from("\\Deleted"),
            Flag::Draft => String::from("\\Draft"),
            Flag::Custom(flag) => flag.clone(),
        }
    }
}

impl TryFrom<&imap::types::Flag<'_>> for Flag {
    type Error = Error;

    fn try_from(imap_flag: &imap::types::Flag) -> Result<Self> {
        match imap_flag {
            imap::types::Flag::Seen => Ok(Flag::Seen),
            imap::types::Flag::Answered => Ok(Flag::Answered),
            imap::types::Flag::Flagged => Ok(Flag::Flagged),
            imap::types::Flag::Deleted => Ok(Flag::Deleted),
            imap::types::Flag::Draft => Ok(Flag::Draft),
            unknown => Err(Error::ParseFlagError(unknown.to_string())),
        }
    }
}

impl TryFrom<imap::types::Flag<'_>> for Flag {
    type Error = Error;

    fn try_from(imap_flag: imap::types::Flag) -> Result<Self> {
        Flag::try_from(&imap_flag)
    }
}

impl Into<imap::types::Flag<'static>> for Flag {
    fn into(self) -> imap::types::Flag<'static> {
        match self {
            Flag::Seen => imap::types::Flag::Seen,
            Flag::Answered => imap::types::Flag::Answered,
            Flag::Flagged => imap::types::Flag::Flagged,
            Flag::Deleted => imap::types::Flag::Deleted,
            Flag::Draft => imap::types::Flag::Draft,
            Flag::Custom(flag) => imap::types::Flag::Custom(flag.into()),
        }
    }
}
