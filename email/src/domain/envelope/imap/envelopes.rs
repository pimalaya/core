use crate::{
    backend::imap::{Error, Result},
    Envelope, Envelopes,
};

impl TryFrom<imap::types::Fetches> for Envelopes {
    type Error = Error;

    fn try_from(fetches: imap::types::Fetches) -> Result<Self> {
        fetches
            .iter()
            .rev()
            .map(Envelope::try_from)
            .collect::<Result<Envelopes>>()
    }
}
