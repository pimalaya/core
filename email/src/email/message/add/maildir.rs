use async_trait::async_trait;
use tracing::info;

use super::{AddMessage, Flags};
use crate::{email::error::Error, envelope::SingleId, maildir::MaildirContextSync, AnyResult};

#[derive(Clone)]
pub struct AddMaildirMessage {
    pub ctx: MaildirContextSync,
}

impl AddMaildirMessage {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn AddMessage> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn AddMessage>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddMessage for AddMaildirMessage {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> AnyResult<SingleId> {
        info!("adding maildir message to folder {folder} with flags {flags}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entry = mdir
            .write_cur(
                raw_msg,
                flags
                    .iter()
                    .filter_map(|flag| maildirs::Flag::try_from(flag).ok()),
            )
            .map_err(|err| {
                Error::StoreWithFlagsMaildirError(err, folder.to_owned(), flags.clone())
            })?;

        Ok(SingleId::from(entry.id().unwrap()))
    }
}
