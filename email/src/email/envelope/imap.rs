//! Module dedicated to IMAP email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [imap] crate types.

use std::{collections::HashMap, num::NonZeroU32};

use imap_client::imap_flow::imap_codec::imap_types::{
    core::Vec1,
    fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
};
use once_cell::sync::Lazy;

use crate::{
    envelope::{Envelope, Envelopes},
    flag::Flags,
    message::Message,
};

/// The IMAP fetch items needed to retrieve everything we need to
/// build an envelope: UID, flags and envelope (Message-ID, From, To,
/// Subject, Date).
pub static FETCH_ENVELOPES: Lazy<MacroOrMessageDataItemNames<'static>> = Lazy::new(|| {
    MacroOrMessageDataItemNames::MessageDataItemNames(vec![
        MessageDataItemName::Uid,
        MessageDataItemName::Flags,
        MessageDataItemName::Envelope,
    ])
});

impl Envelopes {
    pub fn from_imap_data_items(fetches: HashMap<NonZeroU32, Vec1<MessageDataItem>>) -> Self {
        fetches
            .values()
            .map(|items| Envelope::from_imap_data_items(items.as_ref()))
            .collect()
    }
}

impl From<Vec<Vec1<MessageDataItem<'_>>>> for Envelopes {
    fn from(fetches: Vec<Vec1<MessageDataItem>>) -> Self {
        fetches
            .iter()
            .map(|items| Envelope::from_imap_data_items(items.as_ref()))
            .collect()
    }
}

impl Envelope {
    pub fn from_imap_data_items(items: &[MessageDataItem]) -> Self {
        let mut id = 0;
        let mut flags = Flags::default();
        let mut msg = Vec::default();

        for item in items {
            match item {
                MessageDataItem::Uid(uid) => {
                    id = uid.get() as usize;
                }
                MessageDataItem::Flags(fetches) => {
                    flags = Flags::from_imap_flag_fetches(fetches.as_ref());
                }
                MessageDataItem::Envelope(envelope) => {
                    if let Some(msg_id) = envelope.message_id.0.as_ref() {
                        msg.extend(b"Message-ID: ");
                        msg.extend(msg_id.as_ref());
                        msg.push(b'\n');
                    }

                    if let Some(date) = envelope.date.0.as_ref() {
                        msg.extend(b"Date: ");
                        msg.extend(date.as_ref());
                        msg.push(b'\n');
                    }

                    let from = envelope
                        .from
                        .iter()
                        .filter_map(|imap_addr| {
                            let mut addr = Vec::default();

                            if let Some(name) = imap_addr.name.0.as_ref() {
                                addr.push(b'"');
                                addr.extend(name.as_ref());
                                addr.push(b'"');
                                addr.push(b' ');
                            }

                            addr.push(b'<');
                            addr.extend(imap_addr.mailbox.0.as_ref()?.as_ref());
                            addr.push(b'@');
                            addr.extend(imap_addr.host.0.as_ref()?.as_ref());
                            addr.push(b'>');

                            Some(addr)
                        })
                        .fold(b"From: ".to_vec(), |mut addrs, addr| {
                            if !addrs.is_empty() {
                                addrs.push(b',')
                            }
                            addrs.extend(addr);
                            addrs
                        });
                    msg.extend(&from);
                    msg.push(b'\n');

                    let to = envelope
                        .to
                        .iter()
                        .filter_map(|imap_addr| {
                            let mut addr = Vec::default();

                            if let Some(name) = imap_addr.name.0.as_ref() {
                                addr.push(b'"');
                                addr.extend(name.as_ref());
                                addr.push(b'"');
                                addr.push(b' ');
                            }

                            addr.push(b'<');
                            addr.extend(imap_addr.mailbox.0.as_ref()?.as_ref());
                            addr.push(b'@');
                            addr.extend(imap_addr.host.0.as_ref()?.as_ref());
                            addr.push(b'>');

                            Some(addr)
                        })
                        .fold(b"To: ".to_vec(), |mut addrs, addr| {
                            if !addrs.is_empty() {
                                addrs.push(b',')
                            }
                            addrs.extend(addr);
                            addrs
                        });
                    msg.extend(&to);
                    msg.push(b'\n');

                    if let Some(subject) = envelope.subject.0.as_ref() {
                        msg.extend(b"Subject: ");
                        msg.extend(subject.as_ref());
                        msg.push(b'\n');
                    }

                    msg.push(b'\n');
                }
                _ => (),
            }
        }

        let msg = Message::from(msg);
        Envelope::from_msg(id, flags, msg)
    }
}
