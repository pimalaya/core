//! Module dedicated to Maildir email envelopes.

use log::{debug, warn};
use maildirpp::{MailEntries, MailEntry};
use rayon::prelude::*;

use crate::{Envelope, Envelopes, Flags, Message};

impl From<MailEntries> for Envelopes {
    fn from(entries: MailEntries) -> Self {
        Envelopes::from_iter(
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(Envelope::from(entry)),
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

impl From<MailEntry> for Envelope {
    fn from(entry: MailEntry) -> Self {
        let msg = Message::from(entry.headers());
        Envelope::from_msg(entry.id(), Flags::from_mdir_entry(&entry), msg)
    }
}
