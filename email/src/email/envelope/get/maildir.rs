use async_trait::async_trait;
use log::{info, trace};
use std::path::PathBuf;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirContextSync, Result};

use super::{Envelope, GetEnvelope};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot find maildir envelope {1} from folder {0}")]
    GetEnvelopeError(PathBuf, Id),
}

#[derive(Clone)]
pub struct GetMaildirEnvelope {
    ctx: MaildirContextSync,
}

impl GetMaildirEnvelope {
    pub fn new(ctx: impl Into<MaildirContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<MaildirContextSync>) -> Box<dyn GetEnvelope> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl GetEnvelope for GetMaildirEnvelope {
    async fn get_envelope(&self, folder: &str, id: &Id) -> Result<Envelope> {
        info!("getting maildir envelope {id} from folder {folder}");

        let session = self.ctx.lock().await;
        let mdir = session.get_maildir_from_folder_name(folder)?;

        let envelope: Envelope = Envelope::from_mdir_entry(
            mdir.find(&id.to_string())
                .ok_or_else(|| Error::GetEnvelopeError(mdir.path().to_owned(), id.clone()))?,
        );
        trace!("maildir envelope: {envelope:#?}");

        Ok(envelope)
    }
}
