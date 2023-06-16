use log::{debug, trace};
use rayon::prelude::*;

use crate::{backend::maildir::Error, Envelope, Envelopes, Flags};

type Result<T> = std::result::Result<T, Error>;

impl TryFrom<maildirpp::MailEntries> for Envelopes {
    type Error = Error;

    fn try_from(entries: maildirpp::MailEntries) -> Result<Self> {
        Ok(Envelopes::from_iter(
            // TODO: clean me, please
            entries
                .collect::<Vec<_>>()
                .into_par_iter()
                .map(|entry| entry.map_err(Error::DecodeEntryError))
                .collect::<Result<Vec<_>>>()?
                .into_par_iter()
                .map(Envelope::try_from)
                .collect::<Result<Vec<_>>>()?,
        ))
    }
}

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
