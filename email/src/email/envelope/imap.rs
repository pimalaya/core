use imap::{
    extensions::sort::SortCriterion,
    types::{Fetch, Fetches},
};
use std::{ops::Deref, result, str::FromStr};

use crate::{backend::imap::Error, Envelope, Envelopes, Flags, Message};

type Result<T> = result::Result<T, Error>;

impl From<Fetches> for Envelopes {
    fn from(fetches: Fetches) -> Self {
        fetches.iter().rev().map(Envelope::from).collect()
    }
}

impl From<&Fetch<'_>> for Envelope {
    fn from(fetch: &Fetch) -> Self {
        let id = fetch
            .uid
            .expect("UID should be included in the IMAP fetch")
            .to_string();

        let flags = Flags::from_imap_fetch(fetch);

        // parse a fake message from the fetch header in order to
        // extract the envelope
        let msg: Message = fetch
            .header()
            .expect("Header should be included in the IMAP fetch")
            .into();

        Envelope::from_msg(id, flags, msg)
    }
}

/// The IMAP envelope sort criteria. It is just a wrapper around
/// [`imap::extensions::sort::SortCriterion`].
pub struct SortCriteria<'a>(Vec<SortCriterion<'a>>);

impl<'a> Deref for SortCriteria<'a> {
    type Target = Vec<SortCriterion<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> FromIterator<SortCriterion<'a>> for SortCriteria<'a> {
    fn from_iter<T: IntoIterator<Item = SortCriterion<'a>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl FromStr for SortCriteria<'_> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        s.split_whitespace()
            .map(|s| match s.trim() {
                "arrival:asc" | "arrival" => Ok(SortCriterion::Arrival),
                "arrival:desc" => Ok(SortCriterion::Reverse(&SortCriterion::Arrival)),
                "cc:asc" | "cc" => Ok(SortCriterion::Cc),
                "cc:desc" => Ok(SortCriterion::Reverse(&SortCriterion::Cc)),
                "date:asc" | "date" => Ok(SortCriterion::Date),
                "date:desc" => Ok(SortCriterion::Reverse(&SortCriterion::Date)),
                "from:asc" | "from" => Ok(SortCriterion::From),
                "from:desc" => Ok(SortCriterion::Reverse(&SortCriterion::From)),
                "size:asc" | "size" => Ok(SortCriterion::Size),
                "size:desc" => Ok(SortCriterion::Reverse(&SortCriterion::Size)),
                "subject:asc" | "subject" => Ok(SortCriterion::Subject),
                "subject:desc" => Ok(SortCriterion::Reverse(&SortCriterion::Subject)),
                "to:asc" | "to" => Ok(SortCriterion::To),
                "to:desc" => Ok(SortCriterion::Reverse(&SortCriterion::To)),
                _ => Err(Error::ParseSortCriterionError(s.to_owned())),
            })
            .collect::<Result<_>>()
    }
}
