use crate::{
    backend::notmuch::{Error, Result},
    Envelope, Envelopes,
};

impl TryFrom<notmuch::Messages> for Envelopes {
    type Error = Error;

    fn try_from(fetches: notmuch::Messages) -> Result<Self> {
        fetches
            .map(Envelope::try_from)
            .collect::<Result<Envelopes>>()
    }
}
