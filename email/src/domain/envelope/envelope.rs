use chrono::{DateTime, FixedOffset, Local, TimeZone};
use log::{trace, warn};
use mail_parser::Message;

use crate::Flags;

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
    pub fn new<N, A>(name: Option<N>, address: A) -> Self
    where
        N: ToString,
        A: ToString,
    {
        Self {
            name: name.map(|name| name.to_string()),
            addr: address.to_string(),
        }
    }

    pub fn new_nameless<A>(address: A) -> Self
    where
        A: ToString,
    {
        Self {
            name: None,
            addr: address.to_string(),
        }
    }
}

/// Represents the message envelope. The envelope is just a message
/// subset, and is mostly used for listings.
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Envelope {
    /// Represents the envelope identifier.
    pub id: String,
    /// Represents the Message-ID header.
    pub message_id: String,
    /// Represents the flags.
    pub flags: Flags,
    /// Represents the first sender.
    pub from: Mailbox,
    /// Represents the Subject header.
    pub subject: String,
    /// Represents the Date header.
    pub date: DateTime<Local>,
}

impl PartialEq for Envelope {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
    }
}

impl From<&[u8]> for Envelope {
    fn from(raw: &[u8]) -> Self {
        let mut envelope = Self::default();

        match Message::parse(raw) {
            None => {
                warn!("cannot parse envelope from headers, skipping it");
                trace!("{:#?}", String::from_utf8_lossy(raw))
            }
            Some(email) => {
                match email.from() {
                    mail_parser::HeaderValue::Address(addr) if addr.address.is_some() => {
                        let name = addr.name.as_ref().map(|name| name.to_string());
                        let email = addr.address.as_ref().map(|name| name.to_string()).unwrap();
                        envelope.from = Mailbox::new(name, email);
                    }
                    mail_parser::HeaderValue::AddressList(addrs)
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
                    mail_parser::HeaderValue::Group(group)
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
                    mail_parser::HeaderValue::GroupList(groups)
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
                        warn!("cannot extract envelope sender from headers, skipping it");
                        trace!("{:#?}", email.from());
                    }
                };

                envelope.subject = email.subject().map(ToOwned::to_owned).unwrap_or_default();

                match email.date() {
                    Some(date) => envelope.set_date(date),
                    None => warn!("cannot extract envelope date from headers, skipping it"),
                };

                envelope.message_id = email
                    .message_id()
                    .map(|message_id| format!("<{message_id}>"))
                    .unwrap_or_else(|| envelope.date.to_rfc3339());
            }
        }

        envelope
    }
}

impl Envelope {
    pub fn set_date(&mut self, date: &mail_parser::DateTime) {
        self.date = {
            let tz_secs = (date.tz_hour as i32) * 3600 + (date.tz_minute as i32) * 60;
            let tz_sign = if date.tz_before_gmt { -1 } else { 1 };

            let tz = match FixedOffset::east_opt(tz_sign * tz_secs) {
                Some(tz) => tz,
                None => {
                    warn!("invalid timezone {} secs, falling back to 0", tz_secs);
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
            .map(|date| date.with_timezone(&Local))
            .earliest()
            .unwrap_or_else(|| {
                warn!("cannot parse date {}, skipping it", date);
                DateTime::default()
            })
        }
    }

    pub fn set_raw_date(&mut self, data: &[u8]) {
        match mail_parser::parsers::MessageStream::new(data).parse_date() {
            mail_parser::HeaderValue::DateTime(ref date) => {
                self.set_date(date);
            }
            _ => {
                warn!(
                    "cannot parse raw date {}, skipping it",
                    String::from_utf8_lossy(&data.to_vec()),
                );
                self.date = DateTime::default();
            }
        };
    }
}
