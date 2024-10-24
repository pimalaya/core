use std::fs;

use async_trait::async_trait;
use tracing::info;

use super::{Messages, PeekMessages};
use crate::{email::error::Error, envelope::Id, notmuch::NotmuchContextSync, AnyResult};

#[derive(Clone)]
pub struct PeekNotmuchMessages {
    ctx: NotmuchContextSync,
}

impl PeekNotmuchMessages {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn PeekMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn PeekMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PeekMessages for PeekNotmuchMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        info!("peeking notmuch messages {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let msgs: Messages = id
            .iter()
            .map(|ids| {
                let path = db
                    .find_message(ids)
                    .map_err(Error::NotMuchFailure)?
                    .ok_or_else(|| {
                        Error::FindEnvelopeEmptyNotmuchError(folder.to_owned(), ids.to_owned())
                    })?
                    .filename()
                    .to_owned();
                let msg = fs::read(path).map_err(Error::FileReadFailure)?;
                Ok(msg)
            })
            .collect::<AnyResult<Vec<_>>>()?
            .into();

        db.close().map_err(Error::NotMuchFailure)?;

        Ok(msgs)
    }
}
