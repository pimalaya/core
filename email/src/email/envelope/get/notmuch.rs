use async_trait::async_trait;
use log::{info, trace};

use crate::{email::error::Error, envelope::Id, notmuch::NotmuchContextSync};

use super::{Envelope, GetEnvelope};

#[derive(Clone)]
pub struct GetNotmuchEnvelope {
    ctx: NotmuchContextSync,
}

impl GetNotmuchEnvelope {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn GetEnvelope> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn GetEnvelope>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl GetEnvelope for GetNotmuchEnvelope {
    async fn get_envelope(&self, folder: &str, id: &Id) -> crate::Result<Envelope> {
        info!("getting notmuch envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let envelope =
            Envelope::from_notmuch_msg(db.find_message(&id.to_string())?.ok_or_else(|| {
                Error::FindEnvelopeEmptyNotmuchError(folder.to_owned(), id.to_string())
            })?);
        trace!("notmuch envelope: {envelope:#?}");

        db.close()?;

        Ok(envelope)
    }
}
