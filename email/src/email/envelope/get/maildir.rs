use async_trait::async_trait;
use tracing::{info, trace};

use super::{Envelope, GetEnvelope};
use crate::{envelope::SingleId, maildir::MaildirContextSync, AnyResult, Error};

#[derive(Clone)]
pub struct GetMaildirEnvelope {
    ctx: MaildirContextSync,
}

impl GetMaildirEnvelope {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn GetEnvelope> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn GetEnvelope>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl GetEnvelope for GetMaildirEnvelope {
    async fn get_envelope(&self, folder: &str, id: &SingleId) -> AnyResult<Envelope> {
        info!("getting maildir envelope {id:?} from folder {folder}");

        let session = self.ctx.lock().await;
        let mdir = session.get_maildir_from_folder_alias(folder)?;

        let entry = mdir.get(id.to_string()).map_err(Error::from)?;
        let envelope = Envelope::try_from(entry)?;
        trace!("maildir envelope: {envelope:#?}");

        Ok(envelope)
    }
}
