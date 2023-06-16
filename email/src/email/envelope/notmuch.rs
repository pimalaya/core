use log::{debug, warn};

use crate::{Envelope, Envelopes, Flag, Message};

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

impl From<notmuch::Message> for Envelope {
    fn from(msg: notmuch::Message) -> Self {
        let id = msg.id();
        // TODO: move this to the flag module
        let flags = msg.tags().flat_map(Flag::try_from).collect();

        let message_id = get_header(&msg, "Message-ID");
        let subject = get_header(&msg, "Subject");
        let from = get_header(&msg, "From");
        let date = get_header(&msg, "Date");
        let headers = [message_id, subject, from, date].join("\r\n") + "\r\n\r\n";

        // parse a fake message from the built header in order to
        // extract the envelope
        let msg: Message = headers.as_bytes().into();

        Envelope::from_msg(id, flags, msg)
    }
}

fn get_header(msg: &notmuch::Message, key: impl AsRef<str>) -> String {
    let key = key.as_ref();
    let val = match msg.header(key) {
        Ok(Some(val)) => val,
        Ok(None) => Default::default(),
        Err(err) => {
            warn!("cannot get header {key} from notmuch message, skipping it: {err}");
            debug!("cannot get header {key} from notmuch message: {err:?}");
            Default::default()
        }
    };
    format!("{key}: {val}")
}
