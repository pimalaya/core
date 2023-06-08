//! IMAP envelope module.
//!
//! This module provides IMAP types and conversion utilities related
//! to the envelope.

use imap::{self, types::Fetch};
use log::{debug, trace};

use crate::{
    backend::imap::{Error, Result},
    Envelope, Flags,
};

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
