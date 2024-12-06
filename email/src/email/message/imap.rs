use imap_client::imap_next::imap_types::fetch::{
    MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName,
};
use once_cell::sync::Lazy;

use super::Message;
use crate::email::{Error, Result};

/// The IMAP fetch items needed to retrieve everything we need to
/// build an envelope: UID, flags and envelope (Message-ID, From, To,
/// Subject, Date).
pub static FETCH_MESSAGES: Lazy<MacroOrMessageDataItemNames<'static>> = Lazy::new(|| {
    MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
        section: None,
        partial: None,
        peek: false,
    }])
});

/// Same as [`FETCH_MESSAGES`], but with peek set a `true`.
pub static PEEK_MESSAGES: Lazy<MacroOrMessageDataItemNames<'static>> = Lazy::new(|| {
    MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
        section: None,
        partial: None,
        peek: true,
    }])
});

impl<'a> TryFrom<&'a [MessageDataItem<'_>]> for Message<'a> {
    type Error = Error;

    fn try_from(items: &'a [MessageDataItem]) -> Result<Self> {
        for item in items {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0.as_ref() {
                    return Ok(Message::from(data.as_ref()));
                }
            }
        }

        Err(Error::ParseEmailEmptyRawError)
    }
}
