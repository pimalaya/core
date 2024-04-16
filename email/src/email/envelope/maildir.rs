//! Module dedicated to Maildir email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [maildirpp] crate types.

use crate::{debug, trace};
use maildirpp::{MailEntries, MailEntry};
use rayon::prelude::*;

use crate::{
    envelope::{Envelope, Envelopes, Flags},
    message::Message,
    search_query::SearchEmailsQuery,
};

impl Envelopes {
    pub fn from_mdir_entries(entries: MailEntries, query: Option<&SearchEmailsQuery>) -> Self {
        Envelopes::from_iter(
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        debug!("cannot parse maildir entry, skipping it: {err}");
                        trace!("{err:?}");
                        None
                    }
                })
                .filter_map(|entry| {
                    let msg_path = entry.path().to_owned();
                    let envelope = Envelope::from_mdir_entry(entry);
                    if let Some(query) = query {
                        query
                            .matches_maildir_search_query(&envelope, msg_path.as_ref())
                            .then_some(envelope)
                    } else {
                        Some(envelope)
                    }
                })
                .collect::<Vec<_>>(),
        )
    }
}

impl Envelope {
    pub fn from_mdir_entry(entry: MailEntry) -> Self {
        let msg = Message::from(entry.headers());
        Envelope::from_msg(entry.id(), Flags::from_mdir_entry(&entry), msg)
    }
}
