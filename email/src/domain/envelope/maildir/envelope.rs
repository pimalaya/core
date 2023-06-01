use log::{debug, trace};

use crate::{
    backend::maildir::{Error, Result},
    Envelope, Flags,
};

impl TryFrom<maildirpp::MailEntry> for Envelope {
    type Error = Error;

    fn try_from(entry: maildirpp::MailEntry) -> Result<Self> {
        debug!("trying to parse envelope from maildir entry");
        let mut envelope: Envelope = entry.headers().into();

        envelope.id = entry.id().to_owned();

        envelope.flags = Flags::from(&entry);

        trace!("maildir envelope: {envelope:#?}");
        Ok(envelope)
    }
}
