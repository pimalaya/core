use std::result;
use thiserror::Error;

use crate::Flag;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse unknown maildir flag {0}")]
    ParseFlagError(char),
}

pub type Result<T> = result::Result<T, Error>;

impl TryFrom<char> for Flag {
    type Error = Error;

    fn try_from(c: char) -> Result<Self> {
        match c {
            'r' | 'R' => Ok(Flag::Answered),
            's' | 'S' => Ok(Flag::Seen),
            't' | 'T' => Ok(Flag::Deleted),
            'd' | 'D' => Ok(Flag::Draft),
            'f' | 'F' => Ok(Flag::Flagged),
            unknown => Err(Error::ParseFlagError(unknown)),
        }
    }
}

impl Into<Option<char>> for &Flag {
    fn into(self) -> Option<char> {
        match self {
            Flag::Answered => Some('R'),
            Flag::Seen => Some('S'),
            Flag::Deleted => Some('T'),
            Flag::Draft => Some('D'),
            Flag::Flagged => Some('F'),
            _ => None,
        }
    }
}

impl Into<Option<char>> for Flag {
    fn into(self) -> Option<char> {
        (&self).into()
    }
}
