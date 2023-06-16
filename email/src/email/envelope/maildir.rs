use log::{debug, warn};
use maildirpp::{MailEntries, MailEntry};
use rayon::prelude::*;
use std::result;

use crate::{backend::maildir::Error, Envelope, Envelopes, Flags};

type Result<T> = result::Result<T, Error>;

impl From<MailEntries> for Envelopes {
    fn from(entries: MailEntries) -> Self {
        Envelopes::from_iter(
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(Envelope::try_from(entry)),
                    Err(err) => {
                        warn!("cannot parse maildir entry, skipping it: {err}");
                        debug!("cannot parse maildir entry: {err:?}");
                        None
                    }
                })
                .filter_map(|envelope| match envelope {
                    Ok(envelope) => Some(envelope),
                    Err(err) => {
                        warn!("cannot parse maildir envelope, skipping it: {err}");
                        debug!("cannot parse maildir envelope: {err:?}");
                        None
                    }
                })
                .collect::<Vec<_>>(),
        )
    }
}

impl TryFrom<MailEntry> for Envelope {
    type Error = Error;

    fn try_from(entry: MailEntry) -> Result<Self> {
        let mut envelope: Envelope = entry.headers().into();

        envelope.id = entry.id().to_owned();

        envelope.flags = Flags::from(&entry);

        Ok(envelope)
    }
}
