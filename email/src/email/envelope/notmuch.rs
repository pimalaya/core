use log::{debug, warn};
use std::result;

use crate::{backend::notmuch::Error, Envelope, Envelopes, Flag};

type Result<T> = result::Result<T, Error>;

impl From<notmuch::Messages> for Envelopes {
    fn from(msgs: notmuch::Messages) -> Self {
        msgs.filter_map(|msg| match Envelope::try_from(msg) {
            Ok(envelope) => Some(envelope),
            Err(err) => {
                warn!("cannot parse imap envelope, skipping it: {err}");
                debug!("cannot parse imap envelope: {err:?}");
                None
            }
        })
        .collect()
    }
}

impl TryFrom<notmuch::Message> for Envelope {
    type Error = Error;

    fn try_from(msg: notmuch::Message) -> Result<Self> {
        let message_id = get_header(&msg, "Message-ID")?;
        let subject = get_header(&msg, "Subject")?;
        let from = get_header(&msg, "From")?;
        let date = get_header(&msg, "Date")?;

        let headers = [message_id, subject, from, date].join("\r\n") + "\r\n\r\n";

        let mut envelope: Envelope = headers.as_bytes().into();

        envelope.id = msg.id().to_string();

        envelope.flags = msg.tags().flat_map(Flag::try_from).collect();

        Ok(envelope)
    }
}

fn get_header(msg: &notmuch::Message, key: impl AsRef<str>) -> Result<String> {
    let val = msg
        .header(key.as_ref())
        .map_err(|err| Error::GetHeaderError(err, key.as_ref().to_string()))?
        .unwrap_or_default()
        .to_string();

    Ok(format!("{key}: {val}", key = key.as_ref()))
}
