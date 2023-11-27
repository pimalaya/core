use async_trait::async_trait;
use log::{debug, info};
use std::path::PathBuf;
use thiserror::Error;

use crate::{maildir::MaildirSessionSync, Result};

use super::{Envelope, GetEnvelope};

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot find envelope {1} from folder {0}")]
    GetEnvelopeError(PathBuf, String),
}

#[derive(Clone)]
pub struct GetEnvelopeMaildir {
    session: MaildirSessionSync,
}

impl GetEnvelopeMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn GetEnvelope>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl GetEnvelope for GetEnvelopeMaildir {
    async fn get_envelope(&self, folder: &str, id: &str) -> Result<Envelope> {
        info!("getting envelope {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        let envelope: Envelope = Envelope::from_mdir_entry(
            mdir.find(id)
                .ok_or_else(|| Error::GetEnvelopeError(mdir.path().to_owned(), id.to_owned()))?,
        );
        debug!("maildir envelope: {envelope:#?}");

        Ok(envelope)
    }
}
