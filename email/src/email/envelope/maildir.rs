//! Module dedicated to Maildir email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [maildirpp] crate types.

use log::{debug, warn};
use maildirpp::{MailEntries, MailEntry};
use rayon::prelude::*;

use crate::{Envelope, Envelopes, Flags, Message};

impl Envelopes {
    pub fn from_mdir_entries(entries: MailEntries) -> Self {
        Envelopes::from_iter(
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(Envelope::from_mdir_entry(entry)),
                    Err(err) => {
                        warn!("cannot parse maildir entry, skipping it: {err}");
                        debug!("cannot parse maildir entry: {err:?}");
                        None
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
