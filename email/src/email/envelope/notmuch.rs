//! Module dedicated to Notmuch email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [notmuch] crate types.

use tracing::debug;

use crate::{
    envelope::{Envelope, Envelopes},
    flag::{Flag, Flags},
    message::Message,
};

impl Envelopes {
    pub fn from_notmuch_msgs(msgs: notmuch::Messages) -> Self {
        msgs.map(Envelope::from_notmuch_msg).collect()
    }
}

impl Envelope {
    pub fn from_notmuch_msg(msg: notmuch::Message) -> Self {
        let id = msg.id();
        let flags = Flags::from(&msg);
        let has_attachment = flags.contains(&Flag::custom("attachment"));

        let message_id = get_header(&msg, "Message-ID");
        let subject = get_header(&msg, "Subject");
        let from = get_header(&msg, "From");
        let date = get_header(&msg, "Date");
        let headers = [message_id, subject, from, date].join("\r\n") + "\r\n\r\n";

        // parse a fake message from the built header in order to
        // extract the envelope
        let msg: Message = headers.as_bytes().into();

        let mut env = Envelope::from_msg(id, flags, msg);
        env.has_attachment = has_attachment;
        env
    }
}

/// Safely extracts a raw header from a [notmuch::Message] header key.
fn get_header(msg: &notmuch::Message, key: impl AsRef<str>) -> String {
    let key = key.as_ref();
    let val = match msg.header(key) {
        Ok(Some(val)) => val,
        Ok(None) => Default::default(),
        Err(_err) => {
            debug!("cannot get header {key} from notmuch message: {_err}");
            debug!("{_err:?}");
            Default::default()
        }
    };
    format!("{key}: {val}")
}
