//! Module dedicated to IMAP email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [imap] crate types.

use imap::{
    extensions::sort::SortCriterion,
    types::{Fetch, Fetches},
};
use log::debug;
use std::{io, ops::Deref, str::FromStr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get uid of imap envelope {0}: uid is missing")]
    GetUidMissingError(u32),
    #[error("cannot get missing envelope {0}")]
    GetEnvelopeMissingError(u32),
}

use crate::{
    envelope::{Envelope, Envelopes},
    flag::Flags,
    message::Message,
    Result,
};

impl Envelopes {
    pub fn from_imap_fetches(fetches: Fetches) -> Self {
        fetches
            .iter()
            .rev()
            .filter_map(|envelope| match Envelope::from_imap_fetch(envelope) {
                Ok(envelope) => Some(envelope),
                Err(err) => {
                    debug!("cannot build imap envelope: {err}");
                    debug!("{err:?}");
                    None
                }
            })
            .collect()
    }
}

impl Envelope {
    pub fn from_imap_fetch(fetch: &Fetch) -> Result<Self> {
        let mut msg = Vec::new();

        let envelope = fetch
            .envelope()
            .ok_or(Error::GetEnvelopeMissingError(fetch.message))?;

        let id = fetch
            .uid
            .ok_or(Error::GetUidMissingError(fetch.message))?
            .to_string();

        let flags = Flags::from_imap_fetch(fetch);

        if let Some(msg_id) = envelope.message_id.as_ref() {
            msg.extend(b"Message-ID: ");
            msg.extend(msg_id.as_ref());
            msg.push(b'\n');
        }

        if let Some(date) = envelope.date.as_ref() {
            msg.extend(b"Date: ");
            msg.extend(date.as_ref());
            msg.push(b'\n');
        }

        if let Some(addrs) = envelope.from.as_ref() {
            let addrs = addrs
                .iter()
                .filter_map(|imap_addr| {
                    let mut addr = Vec::default();

                    if let Some(name) = imap_addr.name.as_ref() {
                        addr.push(b'"');
                        addr.extend(name.iter());
                        addr.push(b'"');
                        addr.push(b' ');
                    }

                    addr.push(b'<');
                    addr.extend(imap_addr.mailbox.as_ref()?.iter());
                    addr.push(b'@');
                    addr.extend(imap_addr.host.as_ref()?.iter());
                    addr.push(b'>');

                    Some(addr)
                })
                .fold(b"From: ".to_vec(), |mut addrs, addr| {
                    if !addrs.is_empty() {
                        addrs.push(b',')
                    }
                    addrs.extend(addr);
                    addrs
                });

            msg.extend(&addrs);
            msg.push(b'\n');
        }

        if let Some(addrs) = envelope.to.as_ref() {
            let addrs = addrs
                .iter()
                .filter_map(|imap_addr| {
                    let mut addr = Vec::default();

                    if let Some(name) = imap_addr.name.as_ref() {
                        addr.push(b'"');
                        addr.extend(name.iter());
                        addr.push(b'"');
                        addr.push(b' ');
                    }

                    addr.push(b'<');
                    addr.extend(imap_addr.mailbox.as_ref()?.iter());
                    addr.push(b'@');
                    addr.extend(imap_addr.host.as_ref()?.iter());
                    addr.push(b'>');

                    Some(addr)
                })
                .fold(b"To: ".to_vec(), |mut addrs, addr| {
                    if !addrs.is_empty() {
                        addrs.push(b',')
                    }
                    addrs.extend(addr);
                    addrs
                });

            msg.extend(&addrs);
            msg.push(b'\n');
        }

        if let Some(subject) = envelope.subject.as_ref() {
            msg.extend(b"Subject: ");
            msg.extend(subject.as_ref());
            msg.push(b'\n');
        }

        msg.push(b'\n');

        let msg = Message::from(msg);
        let envelope = Envelope::from_msg(id, flags, msg);

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
    type Err = crate::Error;

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
                _ => Ok(Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    s.to_owned(),
                ))?),
            })
            .collect::<Result<_>>()
    }
}
