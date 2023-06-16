use imap::types::{Fetch, Fetches};
use log::{debug, trace};
use std::{convert::TryFrom, ops::Deref};

use crate::{backend::imap::Error, Envelope, Envelopes, Flags};

type Result<T> = std::result::Result<T, Error>;

impl TryFrom<Fetches> for Envelopes {
    type Error = Error;

    fn try_from(fetches: Fetches) -> Result<Self> {
        fetches
            .iter()
            .rev()
            .map(Envelope::try_from)
            .collect::<Result<Envelopes>>()
    }
}

impl TryFrom<&Fetch<'_>> for Envelope {
    type Error = Error;

    fn try_from(fetch: &Fetch) -> Result<Self> {
        debug!("trying to parse envelope from imap fetch");

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

        trace!("imap envelope: {envelope:#?}");
        Ok(envelope)
    }
}

pub type ImapSortCriterion<'a> = imap::extensions::sort::SortCriterion<'a>;

/// Represents the message sort criteria. It is just a wrapper around
/// the `imap::extensions::sort::SortCriterion`.
pub struct SortCriteria<'a>(Vec<imap::extensions::sort::SortCriterion<'a>>);

impl<'a> Deref for SortCriteria<'a> {
    type Target = Vec<ImapSortCriterion<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> TryFrom<&'a str> for SortCriteria<'a> {
    type Error = Error;

    fn try_from(criteria_str: &'a str) -> Result<Self> {
        let mut criteria = vec![];
        for criterion_str in criteria_str.split(" ") {
            criteria.push(match criterion_str.trim() {
                "arrival:asc" | "arrival" => Ok(imap::extensions::sort::SortCriterion::Arrival),
                "arrival:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Arrival,
                )),
                "cc:asc" | "cc" => Ok(imap::extensions::sort::SortCriterion::Cc),
                "cc:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Cc,
                )),
                "date:asc" | "date" => Ok(imap::extensions::sort::SortCriterion::Date),
                "date:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Date,
                )),
                "from:asc" | "from" => Ok(imap::extensions::sort::SortCriterion::From),
                "from:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::From,
                )),
                "size:asc" | "size" => Ok(imap::extensions::sort::SortCriterion::Size),
                "size:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Size,
                )),
                "subject:asc" | "subject" => Ok(imap::extensions::sort::SortCriterion::Subject),
                "subject:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::Subject,
                )),
                "to:asc" | "to" => Ok(imap::extensions::sort::SortCriterion::To),
                "to:desc" => Ok(imap::extensions::sort::SortCriterion::Reverse(
                    &imap::extensions::sort::SortCriterion::To,
                )),
                _ => Err(Error::ParseSortCriterionError(criterion_str.to_owned())),
            }?);
        }
        Ok(Self(criteria))
    }
}
