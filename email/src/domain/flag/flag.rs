use std::{result, str::FromStr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse unknown flag {0}")]
    ParseFlagError(String),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the flag variants.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Flag {
    Seen,
    Answered,
    Flagged,
    Deleted,
    Draft,
    Custom(String),
}

impl Flag {
    pub fn custom<F>(flag: F) -> Self
    where
        F: ToString,
    {
        Self::Custom(flag.to_string())
    }
}

impl From<&str> for Flag {
    fn from(s: &str) -> Self {
        match s.trim() {
            seen if seen.eq_ignore_ascii_case("seen") => Flag::Seen,
            answered if answered.eq_ignore_ascii_case("answered") => Flag::Answered,
            replied if replied.eq_ignore_ascii_case("replied") => Flag::Answered,
            flagged if flagged.eq_ignore_ascii_case("flagged") => Flag::Flagged,
            deleted if deleted.eq_ignore_ascii_case("deleted") => Flag::Deleted,
            trashed if trashed.eq_ignore_ascii_case("trashed") => Flag::Deleted,
            draft if draft.eq_ignore_ascii_case("draft") => Flag::Draft,
            flag => Flag::Custom(flag.into()),
        }
    }
}

impl FromStr for Flag {
    type Err = Error;

    fn from_str(slice: &str) -> Result<Self> {
        match slice.trim() {
            seen if seen.eq_ignore_ascii_case("seen") => Ok(Flag::Seen),
            answered if answered.eq_ignore_ascii_case("answered") => Ok(Flag::Answered),
            replied if replied.eq_ignore_ascii_case("replied") => Ok(Flag::Answered),
            flagged if flagged.eq_ignore_ascii_case("flagged") => Ok(Flag::Flagged),
            deleted if deleted.eq_ignore_ascii_case("deleted") => Ok(Flag::Deleted),
            trashed if trashed.eq_ignore_ascii_case("trashed") => Ok(Flag::Deleted),
            draft if draft.eq_ignore_ascii_case("draft") => Ok(Flag::Draft),
            unknown => Err(Error::ParseFlagError(unknown.to_string())),
        }
    }
}

impl TryFrom<String> for Flag {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl ToString for Flag {
    fn to_string(&self) -> String {
        match self {
            Flag::Seen => "seen".into(),
            Flag::Answered => "answered".into(),
            Flag::Flagged => "flagged".into(),
            Flag::Deleted => "deleted".into(),
            Flag::Draft => "draft".into(),
            Flag::Custom(flag) => flag.clone(),
        }
    }
}
