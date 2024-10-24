//! Module dedicated to email envelope flags.
//!
//! This module contains everything to serialize and deserialize email
//! envelope flags.

pub mod add;
pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;
pub mod remove;
pub mod set;
#[cfg(feature = "sync")]
pub mod sync;

use std::{
    collections::BTreeSet,
    fmt,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use tracing::debug;

#[cfg(feature = "sync")]
#[doc(inline)]
pub use self::sync::sync;
use crate::email::error::Error;

/// The email envelope flag.
///
/// A flag is like a tag that can be attached to an email
/// envelope. The concept of flag is the same across backends, but
/// their definition may vary. For example, the flag representing
/// answered emails is called `\Answered` for IMAP backend but is
/// called `R` (replied) for Maildir backend. This implementation
/// tries to be as simple as possible and should fit most of the use
/// cases.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub enum Flag {
    /// Flag used when the email envelope has been opened.
    Seen,

    /// Flag used when the email has been answered.
    Answered,

    /// Flag used as a bookmark. The meaning is specific to the user:
    /// it could be important, starred, to check etc.
    Flagged,

    /// Flag used when the email is marked for deletion.
    Deleted,

    /// Flag used when the email is a draft and is therefore not
    /// complete.
    Draft,

    /// Flag used for all other use cases.
    Custom(String),
}

impl Flag {
    /// Creates a custom flag.
    pub fn custom(flag: impl ToString) -> Self {
        Self::Custom(flag.to_string())
    }
}

/// Parse a flag from a string. If the string does not match any of
/// the existing variant, it is considered as custom.
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
            draft if draft.eq_ignore_ascii_case("draft") => Flag::Draft,
            flag => Flag::Custom(flag.into()),
        }
    }
}

/// Parse a flag from a string. If the string does not match any of
/// the existing variant, it returns an error.
impl FromStr for Flag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        match s.trim() {
            seen if seen.eq_ignore_ascii_case("seen") => Ok(Flag::Seen),
            answered if answered.eq_ignore_ascii_case("answered") => Ok(Flag::Answered),
            replied if replied.eq_ignore_ascii_case("replied") => Ok(Flag::Answered),
            flagged if flagged.eq_ignore_ascii_case("flagged") => Ok(Flag::Flagged),
            deleted if deleted.eq_ignore_ascii_case("deleted") => Ok(Flag::Deleted),
            trashed if trashed.eq_ignore_ascii_case("trashed") => Ok(Flag::Deleted),
            draft if draft.eq_ignore_ascii_case("draft") => Ok(Flag::Draft),
            drafts if drafts.eq_ignore_ascii_case("drafts") => Ok(Flag::Draft),
            unknown => Err(Error::ParseFlagError(unknown.to_string())),
        }
    }
}

/// Alias for [`FromStr`].
impl TryFrom<String> for Flag {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        value.parse()
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let flag = match self {
            Flag::Seen => "seen".into(),
            Flag::Answered => "answered".into(),
            Flag::Flagged => "flagged".into(),
            Flag::Deleted => "deleted".into(),
            Flag::Draft => "draft".into(),
            Flag::Custom(flag) => flag.clone(),
        };
        write!(f, "{flag}")
    }
}

/// The set of email envelope flags.
///
/// The list of flags that can be attached to an email envelope. It
/// uses a [`std::collections::HashSet`] to prevent duplicates.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Flags(BTreeSet<Flag>);

impl Hash for Flags {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut flags = Vec::from_iter(self.iter());
        flags.sort_by(|a, b| a.partial_cmp(b).unwrap());
        flags.hash(state)
    }
}

impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, flag) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{flag}")?;
        }
        Ok(())
    }
}

impl Deref for Flags {
    type Target = BTreeSet<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Flags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&str> for Flags {
    fn from(s: &str) -> Self {
        s.split_whitespace()
            .filter_map(|flag| match flag.parse() {
                Ok(flag) => Some(flag),
                Err(err) => {
                    debug!("cannot parse flag {flag}, skipping it: {err}");
                    debug!("{err:?}");
                    None
                }
            })
            .collect()
    }
}

impl From<String> for Flags {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl FromStr for Flags {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(Flags(
            s.split_whitespace()
                .map(|flag| flag.parse())
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl FromIterator<Flag> for Flags {
    fn from_iter<T: IntoIterator<Item = Flag>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<Flags> for Vec<String> {
    fn from(val: Flags) -> Self {
        val.iter().map(|flag| flag.to_string()).collect()
    }
}
