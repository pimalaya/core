//! Module dedicated to email envelopes.
//!
//! The email envelope is composed of an identifier, some
//! [flags](self::Flags), and few headers taken from the email
//! [message](crate::Message).

pub mod address;
pub mod config;
pub mod flag;
pub mod get;
pub mod id;
#[cfg(feature = "imap")]
pub mod imap;
pub mod list;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(feature = "thread")]
pub mod thread;
#[cfg(feature = "watch")]
pub mod watch;

#[cfg(feature = "thread")]
use std::collections::HashMap;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::{Deref, DerefMut},
    vec,
};

use chrono::{DateTime, FixedOffset, Local};
#[cfg(feature = "thread")]
use petgraph::graphmap::DiGraphMap;
use tracing::{debug, trace};

#[doc(inline)]
pub use self::{
    address::Address,
    flag::{Flag, Flags},
    id::{Id, MultipleIds, SingleId},
};
use crate::{
    account::config::AccountConfig, date::from_mail_parser_to_chrono_datetime, message::Message,
};

/// The email envelope.
///
/// The email envelope is composed of an identifier, some
/// [flags](self::Flags), and few headers taken from the email
/// [message](crate::Message).
#[derive(Clone, Debug, Default, Eq, Ord, PartialOrd)]
pub struct Envelope {
    /// The shape of the envelope identifier may vary depending on the backend.
    /// For IMAP backend, it is an stringified auto-incremented integer.
    /// For Notmuch backend it is a Git-like hash.
    // TODO: replace me with SingleId
    pub id: String,
    /// The Message-ID header from the email message.
    pub message_id: String,
    /// The In-Reply-To header from the email message.
    pub in_reply_to: Option<String>,
    /// The envelope flags.
    pub flags: Flags,
    /// The first address from the email message header From.
    pub from: Address,
    /// The first address from the email message header To.
    pub to: Address,
    /// The Subject header from the email message.
    pub subject: String,
    /// The Date header from the email message.
    pub date: DateTime<FixedOffset>,

    /// True if the current envelope contains at least one attachment.
    ///
    /// An attachment is defined here as a MIME part that is not a
    /// `text/*`.
    pub has_attachment: bool,
}

impl Envelope {
    /// Build an envelope from an identifier, some
    /// [flags](self::Flags) and a [message](super::Message).
    pub fn from_msg(id: impl ToString, flags: Flags, msg: Message) -> Envelope {
        let mut envelope = Envelope {
            id: id.to_string(),
            flags,
            ..Default::default()
        };

        if let Ok(msg) = msg.parsed() {
            match msg.from() {
                Some(mail_parser::Address::List(addrs))
                    if !addrs.is_empty() && addrs[0].address.is_some() =>
                {
                    let name = addrs[0].name.as_ref().map(|name| name.to_string());
                    let email = addrs[0]
                        .address
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap();
                    envelope.from = Address::new(name, email);
                }
                Some(mail_parser::Address::Group(groups))
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
                    envelope.from = Address::new(name, email)
                }
                _ => {
                    trace!("cannot extract envelope sender from message header, skipping it");
                }
            };

            match msg.to() {
                Some(mail_parser::Address::List(addrs))
                    if !addrs.is_empty() && addrs[0].address.is_some() =>
                {
                    let name = addrs[0].name.as_ref().map(|name| name.to_string());
                    let email = addrs[0]
                        .address
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap();
                    envelope.to = Address::new(name, email);
                }
                Some(mail_parser::Address::Group(groups))
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
                    envelope.to = Address::new(name, email)
                }
                _ => {
                    trace!("cannot extract envelope recipient from message header, skipping it");
                }
            };

            envelope.subject = msg.subject().map(ToOwned::to_owned).unwrap_or_default();

            match msg.date() {
                Some(date) => envelope.set_date(date),
                None => {
                    trace!("cannot extract envelope date from message header, skipping it")
                }
            };

            envelope.message_id = msg
                .message_id()
                .map(|mid| format!("<{mid}>"))
                // NOTE: this is useful for the sync to prevent
                // messages without Message-ID to still being
                // synchronized.
                .unwrap_or_else(|| {
                    let mut hasher = DefaultHasher::new();
                    envelope.date.to_string().hash(&mut hasher);
                    format!("<{:x}@generated>", hasher.finish())
                });

            envelope.in_reply_to = msg.in_reply_to().as_text().map(|mid| format!("<{mid}>"));
        } else {
            trace!("cannot parse message header, skipping it");
        };

        envelope
    }

    pub fn set_some_from(&mut self, addr: Option<Address>) {
        if let Some(addr) = addr {
            self.from = addr;
        }
    }

    pub fn set_some_to(&mut self, addr: Option<Address>) {
        if let Some(addr) = addr {
            self.to = addr;
        }
    }

    pub fn set_some_date(&mut self, date: Option<&mail_parser::DateTime>) {
        if let Some(date) = date {
            self.set_date(date)
        }
    }

    /// Transform a [`mail_parser::DateTime`] into a fixed offset [`chrono::DateTime`]
    /// and add it to the current envelope.
    pub fn set_date(&mut self, date: &mail_parser::DateTime) {
        self.date = from_mail_parser_to_chrono_datetime(date).unwrap_or_else(|| {
            debug!("cannot parse envelope date {date}, skipping it");
            DateTime::default()
        });
    }

    /// Format the envelope date according to the datetime format and
    /// timezone from the [account configuration](crate::AccountConfig).
    pub fn format_date(&self, config: &AccountConfig) -> String {
        let fmt = config.get_envelope_list_datetime_fmt();

        let date = if config.has_envelope_list_datetime_local_tz() {
            self.date.with_timezone(&Local).format(&fmt)
        } else {
            self.date.format(&fmt)
        };

        date.to_string()
    }

    /// Build a message from the current envelope.
    ///
    /// The message is just composed of two headers and contains no
    /// content. It is mostly used by the synchronization to cache
    /// envelopes.
    #[cfg(feature = "sync")]
    pub fn to_sync_cache_msg(&self) -> String {
        let id = &self.message_id;
        let date = self.date.to_rfc2822();
        format!("Message-ID: {id}\nDate: {date}\n\n")
    }

    #[cfg(feature = "thread")]
    pub fn as_threaded(&self) -> ThreadedEnvelope {
        ThreadedEnvelope {
            id: self.id.as_str(),
            message_id: self.message_id.as_str(),
            subject: self.subject.as_str(),
            from: match self.from.name.as_ref() {
                Some(name) => name.as_str(),
                None => self.from.addr.as_str(),
            },
            date: self.date,
        }
    }
}

// NOTE: this is useful for the sync, not sure how relevant it is for
// the rest.
impl PartialEq for Envelope {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}

impl Hash for Envelope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message_id.hash(state);
    }
}

/// The list of email envelopes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Envelopes(Vec<Envelope>);

impl IntoIterator for Envelopes {
    type IntoIter = vec::IntoIter<Self::Item>;
    type Item = Envelope;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Envelopes> for Vec<Envelope> {
    fn from(val: Envelopes) -> Self {
        val.0
    }
}

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

#[cfg(feature = "thread")]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialOrd)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ThreadedEnvelope<'a> {
    pub id: &'a str,
    pub message_id: &'a str,
    pub from: &'a str,
    pub subject: &'a str,
    pub date: DateTime<FixedOffset>,
}

#[cfg(feature = "thread")]
impl ThreadedEnvelope<'_> {
    /// Format the envelope date according to the datetime format and
    /// timezone from the [account configuration](crate::AccountConfig).
    pub fn format_date(&self, config: &AccountConfig) -> String {
        let fmt = config.get_envelope_list_datetime_fmt();

        let date = if config.has_envelope_list_datetime_local_tz() {
            self.date.with_timezone(&Local).format(&fmt)
        } else {
            self.date.format(&fmt)
        };

        date.to_string()
    }
}

#[cfg(feature = "thread")]
impl PartialEq for ThreadedEnvelope<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}

#[cfg(feature = "thread")]
impl Hash for ThreadedEnvelope<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.message_id.hash(state);
    }
}

#[cfg_attr(feature = "thread", ouroboros::self_referencing)]
#[derive(Debug)]
pub struct ThreadedEnvelopes {
    #[cfg(feature = "thread")]
    inner: std::collections::HashMap<String, Envelope>,
    #[cfg(feature = "thread")]
    #[borrows(inner)]
    #[covariant]
    graph: DiGraphMap<ThreadedEnvelope<'this>, u8>,
}

#[cfg(feature = "thread")]
impl ThreadedEnvelopes {
    pub fn build(
        envelopes: HashMap<String, Envelope>,
        f: impl Fn(&HashMap<String, Envelope>) -> DiGraphMap<ThreadedEnvelope, u8>,
    ) -> Self {
        ThreadedEnvelopes::new(envelopes, f)
    }

    pub fn map(&self) -> &HashMap<String, Envelope> {
        self.borrow_inner()
    }

    pub fn graph(&self) -> &DiGraphMap<ThreadedEnvelope, u8> {
        self.borrow_graph()
    }
}

#[cfg(all(feature = "thread", feature = "derive"))]
impl serde::Serialize for ThreadedEnvelopes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let count = self.graph().all_edges().count();
        let mut seq = serializer.serialize_seq(Some(count))?;

        for ref edge in self.graph().all_edges() {
            seq.serialize_element(edge)?;
        }

        serde::ser::SerializeSeq::end(seq)
    }
}

#[cfg(all(feature = "thread", feature = "derive"))]
impl<'de> serde::Deserialize<'de> for ThreadedEnvelopes {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        todo!()
    }
}
