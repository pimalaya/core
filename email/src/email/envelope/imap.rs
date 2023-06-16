use imap::{
    extensions::sort::SortCriterion,
    types::{Fetch, Fetches},
};
use log::{debug, warn};
use std::{convert::TryFrom, ops::Deref, result, str::FromStr};

use crate::{backend::imap::Error, Envelope, Envelopes, Flags};

type Result<T> = result::Result<T, Error>;

impl From<Fetches> for Envelopes {
    fn from(fetches: Fetches) -> Self {
        fetches
            .iter()
            .rev()
            .filter_map(|fetch| match Envelope::try_from(fetch) {
                Ok(envelope) => Some(envelope),
                Err(err) => {
                    warn!("cannot parse imap envelope, skipping it: {err}");
                    debug!("cannot parse imap envelope: {err:?}");
                    None
                }
            })
            .collect()
    }
}

impl TryFrom<&Fetch<'_>> for Envelope {
    type Error = Error;

    fn try_from(fetch: &Fetch) -> Result<Self> {
        let id = fetch
            .uid
            .ok_or_else(|| Error::GetUidError(fetch.message))?
            .to_string();

        let mut envelope: Envelope = fetch
            .header()
            .ok_or(Error::GetHeadersFromFetchError(id.clone()))?
            .into();

        envelope.id = id;

        envelope.flags = Flags::from(fetch.flags());

        Ok(envelope)
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
