pub mod flag;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;

use chrono::{DateTime, FixedOffset, Local, TimeZone};
use log::warn;
use mail_parser::HeaderValue;
use std::ops::{Deref, DerefMut};

use crate::{AccountConfig, Message};

pub use self::flag::{Flag, Flags};

/// Wrapper around the list of envelopes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Envelopes(Vec<Envelope>);

impl Deref for Envelopes {
    type Target = Vec<Envelope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Envelopes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<Envelope> for Envelopes {
    fn from_iter<T: IntoIterator<Item = Envelope>>(iter: T) -> Self {
        Envelopes(iter.into_iter().collect())
    }
}

// fn date<S: Serializer>(date: &DateTime<Local>, s: S) -> Result<S::Ok, S::Error> {
//     s.serialize_str(&date.to_rfc3339())
// }

// #[derive(Clone, Debug, Default, Eq, PartialEq)]
// pub struct Mailboxes(Vec<Mailbox>);

// impl ops::Deref for Mailboxes {
//     type Target = Vec<Mailbox>;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl ops::DerefMut for Mailboxes {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

// impl ToString for Mailboxes {
//     fn to_string(&self) -> String {
//         self.iter().fold(String::new(), |mut mboxes, mbox| {
//             if !mboxes.is_empty() {
//                 mboxes.push_str(", ")
//             }
//             mboxes.push_str(&mbox.to_string());
//             mboxes
//         })
//     }
// }

#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Mailbox {
    pub name: Option<String>,
    pub addr: String,
}

impl PartialEq for Mailbox {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl ToString for Mailbox {
    fn to_string(&self) -> String {
        match &self.name {
            Some(name) => format!("{name} <{}>", self.addr),
            None => self.addr.clone(),
        }
    }
}

impl Mailbox {
    pub fn new(name: Option<impl ToString>, address: impl ToString) -> Self {
        Self {
            name: name.map(|name| name.to_string()),
            addr: address.to_string(),
        }
    }

    pub fn new_nameless(address: impl ToString) -> Self {
        Self {
            name: None,
            addr: address.to_string(),
        }
    }
}

/// The email's envelope is composed of an identifier, some flags, and
/// few headers taken from the email's content (message).
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Envelope {
    /// The shape of the envelope identifier may differ depending on the backend.
    /// For IMAP backend, it is an stringified auto-incremented integer.
    /// For Notmuch backend it is a stringified hash.
    pub id: String,
    /// The Message-ID header from the email's content (message).
    pub message_id: String,
    /// The envelope flags.
    pub flags: Flags,
    /// The From header from the email's content (message).
    pub from: Mailbox,
    /// The Subject header from the email's content (message).
    pub subject: String,
    /// The Date header from the email's content (message).
    pub date: DateTime<FixedOffset>,
}

impl Envelope {
    /// Parse an envelope from an identifier, some flags and a message.
    pub fn from_msg(id: impl ToString, flags: Flags, msg: Message) -> Envelope {
        let mut envelope = Envelope {
            id: id.to_string(),
            flags,
            ..Default::default()
        };

        if let Ok(msg) = msg.parsed() {
            match msg.from() {
                HeaderValue::Address(addr) if addr.address.is_some() => {
                    let name = addr.name.as_ref().map(|name| name.to_string());
                    let email = addr.address.as_ref().map(|name| name.to_string()).unwrap();
                    envelope.from = Mailbox::new(name, email);
                }
                HeaderValue::AddressList(addrs)
                    if !addrs.is_empty() && addrs[0].address.is_some() =>
                {
                    let name = addrs[0].name.as_ref().map(|name| name.to_string());
                    let email = addrs[0]
                        .address
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap();
                    envelope.from = Mailbox::new(name, email);
                }
                HeaderValue::Group(group)
                    if !group.addresses.is_empty() && group.addresses[0].address.is_some() =>
                {
                    let name = group.name.as_ref().map(|name| name.to_string());
                    let email = group.addresses[0]
                        .address
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap();
                    envelope.from = Mailbox::new(name, email)
                }
                HeaderValue::GroupList(groups)
                    if !groups.is_empty()
                        && !groups[0].addresses.is_empty()
                        && groups[0].addresses[0].address.is_some() =>
                {
                    let name = groups[0].name.as_ref().map(|name| name.to_string());
                    let email = groups[0].addresses[0]
                        .address
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap();
                    envelope.from = Mailbox::new(name, email)
                }
                _ => {
                    warn!("cannot extract envelope sender from message header, skipping it");
                }
            };

            envelope.subject = msg.subject().map(ToOwned::to_owned).unwrap_or_default();

            match msg.date() {
                Some(date) => envelope.set_date(date),
                None => warn!("cannot extract envelope date from message header, skipping it"),
            };

            envelope.message_id = msg
                .message_id()
                .map(|message_id| format!("<{message_id}>"))
                // NOTE: this is useful for the sync to prevent
                // messages without Message-ID to still being
                // synchronized.
                .unwrap_or_else(|| envelope.date.to_rfc3339());
        };

        envelope
    }

    /// Transform a [`mail_parser::DateTime`] into a fixed offset [`chrono::DateTime`]
    /// and add it to the current envelope.
    pub fn set_date(&mut self, date: &mail_parser::DateTime) {
        self.date = {
            let tz_secs = (date.tz_hour as i32) * 3600 + (date.tz_minute as i32) * 60;
            let tz_sign = if date.tz_before_gmt { -1 } else { 1 };

            let tz = match FixedOffset::east_opt(tz_sign * tz_secs) {
                Some(tz) => tz,
                None => {
                    warn!("invalid timezone seconds {tz_secs}, falling back to 0");
                    FixedOffset::east_opt(0).unwrap()
                }
            };

            tz.with_ymd_and_hms(
                date.year as i32,
                date.month as u32,
                date.day as u32,
                date.hour as u32,
                date.minute as u32,
                date.second as u32,
            )
            .earliest()
            .unwrap_or_else(|| {
                warn!("cannot parse envelope date {date}, skipping it");
                DateTime::default()
            })
        }
    }

    /// Format the envelope date according to the [`crate::AccountConfig`]
    /// datetime format and timezone.
    pub fn format_date(&self, config: &AccountConfig) -> String {
        let fmt = config.email_listing_datetime_fmt();

        let date = if config.email_listing_datetime_local_tz() {
            self.date.with_timezone(&Local).format(&fmt)
        } else {
            self.date.format(&fmt)
        };

        date.to_string()
    }
}

// NOTE: this is useful for the sync, not sure how relevant it is for
// the rest.
impl PartialEq for Envelope {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}
