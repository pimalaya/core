//! Module dedicated to email envelopes.
//!
//! The email envelope is composed of an identifier, some
//! [flags](self::Flags), and few headers taken from the email
//! [message](crate::Message).

pub mod address;
pub mod flag;
pub mod get;
pub mod id;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod list;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;

use chrono::{DateTime, FixedOffset, Local, TimeZone};
use log::warn;
use std::ops::{Deref, DerefMut};

use crate::account::AccountConfig;

use super::Message;

#[doc(inline)]
pub use self::{
    address::Address,
    flag::{Flag, Flags},
    get::GetEnvelope,
    id::{Id, MultipleIds, SingleId},
    list::ListEnvelopes,
};

/// The email envelope.
///
/// The email envelope is composed of an identifier, some
/// [flags](self::Flags), and few headers taken from the email
/// [message](crate::Message).
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Envelope {
    /// The shape of the envelope identifier may vary depending on the backend.
    /// For IMAP backend, it is an stringified auto-incremented integer.
    /// For Notmuch backend it is a Git-like hash.
    pub id: String,
    /// The Message-ID header from the email message.
    pub message_id: String,
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
        } else {
            warn!("cannot parse message header, skipping it");
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

    /// Format the envelope date according to the datetime format and
    /// timezone from the [account configuration](crate::AccountConfig).
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

/// The list of email envelopes.
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
