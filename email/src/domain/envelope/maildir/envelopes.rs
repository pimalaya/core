//! Maildir mailbox module.
//!
//! This module provides Maildir types and conversion utilities
//! related to the envelope.
use rayon::prelude::*;

use crate::{
    backend::maildir::{Error, Result},
    Envelope, Envelopes,
};

impl TryFrom<maildir::MailEntries> for Envelopes {
    type Error = Error;

    fn try_from(entries: maildir::MailEntries) -> Result<Self> {
        Ok(Envelopes::from_iter(
            // TODO: clean me please
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .map(|entry| entry.map_err(Error::DecodeEntryError))
                .collect::<Result<Vec<_>>>()?
                .into_par_iter()
                .map(Envelope::try_from)
                .collect::<Result<Vec<_>>>()?,
        ))
    }
}
