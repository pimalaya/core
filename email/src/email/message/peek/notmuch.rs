use async_trait::async_trait;
use log::info;
use std::fs;
use thiserror::Error;

use crate::{envelope::Id, notmuch::NotmuchContextSync, Result};

use super::{Messages, PeekMessages};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find notmuch envelope {1} from folder {0}")]
    FindEnvelopeEmptyError(String, String),
}

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
    async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        info!("peeking notmuch messages {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let msgs: Messages = id
            .iter()
            .map(|id| {
                let path = db
                    .find_message(&id)?
                    .ok_or_else(|| {
                        Error::FindEnvelopeEmptyError(folder.to_owned(), id.to_string())
                    })?
                    .filename()
                    .to_owned();
                let msg = fs::read(path)?;
                Ok(msg)
            })
            .collect::<Result<Vec<_>>>()?
            .into();

        db.close()?;

        Ok(msgs)
    }
}
