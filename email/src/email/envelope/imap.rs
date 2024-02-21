//! Module dedicated to IMAP email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [imap] crate types.

use imap::{
    extensions::sort::SortCriterion,
    types::{Fetch, Fetches},
};
use log::debug;
use mail_parser::parsers::MessageStream;
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
    Result,
};

use super::Address;

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
        let mut envelope = Envelope::default();

        let fetch_envelope = fetch
            .envelope()
            .ok_or(Error::GetEnvelopeMissingError(fetch.message))?;

        envelope.id = fetch
            .uid
            .ok_or(Error::GetUidMissingError(fetch.message))?
            .to_string();

        envelope.flags = Flags::from_imap_fetch(fetch);

        envelope.subject = fetch_envelope
            .subject
            .as_ref()
            .map(|subject| String::from_utf8_lossy(subject))
            .unwrap_or_default()
            .to_string();

        envelope.set_some_from(fetch_envelope.from.as_ref().and_then(find_first_address));

        envelope.set_some_to(fetch_envelope.to.as_ref().and_then(find_first_address));

        envelope.set_some_date(
            fetch_envelope
                .date
                .as_ref()
                .and_then(|date| {
                    mail_parser::parsers::MessageStream::new(date)
                        .parse_date()
                        .into_datetime()
                })
                .as_ref(),
        );

        envelope.message_id = fetch_envelope
            .message_id
            .as_ref()
            .and_then(|msg_id| {
                // needed by mail-parser, otherwise it is parsed as
                // empty value.
                let mut msg_id = msg_id.to_vec();
                msg_id.push(b'\n');

                let msg_id = MessageStream::new(&msg_id).parse_id().into_text()?;
                Some(format!("<{msg_id}>"))
            })
            .unwrap_or_else(|| {
                let date_hash = md5::compute(envelope.date.to_string());
                format!("<{date_hash:x}@generated>")
            });

        Ok(envelope)
    }
}

fn find_first_address(imap_addrs: &Vec<imap_proto::Address>) -> Option<Address> {
    let addr = imap_addrs.iter().find_map(|imap_addr| {
        let mut addr = Vec::default();

        if let Some(name) = imap_addr.name.as_ref() {
            addr.extend(['"' as u8]);
            addr.extend(name.iter());
            addr.extend(['"' as u8, ' ' as u8]);
        }

        addr.extend(['<' as u8]);
        addr.extend(imap_addr.mailbox.as_ref()?.iter());
        addr.extend(['@' as u8]);
        addr.extend(imap_addr.host.as_ref()?.iter());
        addr.extend(['>' as u8]);

        Some(addr)
    });

    if let Some(addr) = addr {
        let addr = mail_parser::parsers::MessageStream::new(&addr)
            .parse_address()
            .into_address();

        match addr {
            None => (),
            Some(mail_parser::Address::List(addrs)) => {
                if let Some(addr) = addrs.iter().find(|a| a.address.is_some()) {
                    let name = addr.name.as_ref();
                    return Some(Address::new(name, addr.address().unwrap()));
                }
            }
            Some(mail_parser::Address::Group(groups)) => {
                if let Some(g) = groups.first() {
                    if let Some(addr) = g.addresses.iter().find(|a| a.address.is_some()) {
                        let name = addr.name.as_ref();
                        return Some(Address::new(name, addr.address().unwrap()));
                    }
                }
            }
        }
    }

    None
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
