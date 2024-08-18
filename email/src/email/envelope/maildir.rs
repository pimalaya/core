//! Module dedicated to Maildir email envelopes.
//!
//! This module contains envelope-related mapping functions from the
//! [maildirpp] crate types.

use maildirs::MaildirEntry;
use rayon::prelude::*;

use crate::{
    envelope::{Envelope, Envelopes, Flags},
    message::Message,
    search_query::SearchEmailsQuery,
    Error, Result,
};

impl Envelopes {
    pub fn from_mdir_entries(
        entries: impl Iterator<Item = MaildirEntry>,
        query: Option<&SearchEmailsQuery>,
    ) -> Self {
        Envelopes::from_iter(
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|entry| {
                    let msg_path = entry.path().to_owned();
                    let envelope = Envelope::try_from(entry).ok()?;
                    if let Some(query) = query {
                        query
                            .matches_maildir_search_query(&envelope, msg_path.as_ref())
                            .then_some(envelope)
                    } else {
                        Some(envelope)
                    }
                })
                .collect::<Vec<_>>(),
        )
    }
}

impl TryFrom<MaildirEntry> for Envelope {
    type Error = Error;

    fn try_from(entry: MaildirEntry) -> Result<Self> {
        let id = entry.id()?.to_owned();
        let msg = Message::from(entry.read()?);

        let has_attachment = {
            let attachments = msg.attachments();

            match attachments {
                Ok(attachments) => !attachments.is_empty(),
                Err(_) => false,
            }
        };

        let flags = Flags::try_from(entry)?;
        let mut env = Envelope::from_msg(id, flags, msg);
        env.has_attachment = has_attachment;
        Ok(env)
    }
}
