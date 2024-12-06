//! Module dedicated to IMAP email envelope flags.
//!
//! This module contains flag-related mapping functions from the
//! [imap] crate types.

use std::fmt;

use imap_client::imap_next::imap_types::{
    error::ValidationError,
    flag::{Flag as ImapFlag, FlagFetch},
    search::SearchKey,
};
use tracing::{debug, trace};

use super::{Flag, Flags};
use crate::email::error::Error;

impl Flags {
    pub fn from_imap_flag_fetches(fetches: &[FlagFetch<'_>]) -> Self {
        Flags::from_iter(fetches.iter().filter_map(|fetch| {
            match Flag::try_from_imap_fetch(fetch) {
                Ok(flag) => Some(flag),
                Err(_err) => {
                    trace!("{_err:?}");
                    None
                }
            }
        }))
    }

    pub fn to_imap_flags_iter(
        &self,
    ) -> impl IntoIterator<Item = ImapFlag<'static>> + fmt::Debug + Clone + '_ {
        self.iter()
            .filter_map(|flag| match flag.clone().try_into() {
                Ok(flag) => Some(flag),
                Err(_err) => {
                    debug!("cannot serialize IMAP flag {flag}: {_err}");
                    trace!("{_err:?}");
                    None
                }
            })
    }
}

impl Flag {
    pub fn to_imap_string(&self) -> String {
        match self {
            Flag::Seen => String::from("\\Seen"),
            Flag::Answered => String::from("\\Answered"),
            Flag::Flagged => String::from("\\Flagged"),
            Flag::Deleted => String::from("\\Deleted"),
            Flag::Draft => String::from("\\Draft"),
            Flag::Custom(flag) => flag.clone(),
        }
    }

    pub fn try_from_imap_fetch(fetch: &FlagFetch<'_>) -> Result<Self, Error> {
        match fetch {
            FlagFetch::Flag(ImapFlag::Seen) => Ok(Flag::Seen),
            FlagFetch::Flag(ImapFlag::Answered) => Ok(Flag::Answered),
            FlagFetch::Flag(ImapFlag::Flagged) => Ok(Flag::Flagged),
            FlagFetch::Flag(ImapFlag::Deleted) => Ok(Flag::Deleted),
            FlagFetch::Flag(ImapFlag::Draft) => Ok(Flag::Draft),
            FlagFetch::Flag(flag) => Err(Error::ParseFlagImapError(flag.to_string())),
            FlagFetch::Recent => Err(Error::ParseFlagImapError("\\Recent".into())),
        }
    }
}

impl TryFrom<Flag> for ImapFlag<'static> {
    type Error = ValidationError;

    fn try_from(flag: Flag) -> Result<ImapFlag<'static>, Self::Error> {
        Ok(match flag {
            Flag::Seen => ImapFlag::Seen,
            Flag::Answered => ImapFlag::Answered,
            Flag::Flagged => ImapFlag::Flagged,
            Flag::Deleted => ImapFlag::Deleted,
            Flag::Draft => ImapFlag::Draft,
            Flag::Custom(flag) => ImapFlag::Keyword(flag.try_into()?),
        })
    }
}

impl<'a> TryFrom<Flag> for SearchKey<'a> {
    type Error = ValidationError;

    fn try_from(flag: Flag) -> Result<SearchKey<'a>, Self::Error> {
        Ok(match flag {
            Flag::Seen => SearchKey::Seen,
            Flag::Answered => SearchKey::Answered,
            Flag::Flagged => SearchKey::Flagged,
            Flag::Deleted => SearchKey::Deleted,
            Flag::Draft => SearchKey::Draft,
            Flag::Custom(flag) => SearchKey::Keyword(flag.try_into()?),
        })
    }
}
