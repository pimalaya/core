//! Notmuch folder module.
//!
//! This module provides Notmuch types and conversion utilities
//! related to the envelope

use log::{debug, trace};
use notmuch;

use crate::{
    backend::notmuch::{Error, Result},
    Envelope, Flag,
};

impl TryFrom<notmuch::Message> for Envelope {
    type Error = Error;

    fn try_from(msg: notmuch::Message) -> Result<Self> {
        debug!("trying to parse envelope from notmuch message");

        let message_id = get_header(&msg, "Message-ID")?;
        let subject = get_header(&msg, "Subject")?;
        let from = get_header(&msg, "From")?;
        let date = get_header(&msg, "Date")?;

        let headers = [message_id, subject, from, date].join("\r\n") + "\r\n\r\n";

        let mut envelope: Envelope = headers.as_bytes().into();

        envelope.internal_id = msg.id().to_string();

        envelope.flags = msg.tags().flat_map(Flag::try_from).collect();

        trace!("notmuch envelope: {envelope:#?}");
        Ok(envelope)
    }
}

fn get_header<K>(msg: &notmuch::Message, key: K) -> Result<String>
where
    K: AsRef<str> + ToString,
{
    let val = msg
        .header(key.as_ref())
        .map_err(|err| Error::GetHeaderError(err, key.to_string()))?
        .unwrap_or_default()
        .to_string();

    Ok(format!("{key}: {val}", key = key.as_ref()))
}
