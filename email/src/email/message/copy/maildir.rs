use async_trait::async_trait;
use tracing::info;

use super::CopyMessages;
use crate::{email::error::Error, envelope::Id, maildir::MaildirContextSync, AnyResult};

#[derive(Clone)]
pub struct CopyMaildirMessages {
    ctx: MaildirContextSync,
}

impl CopyMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn CopyMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn CopyMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CopyMessages for CopyMaildirMessages {
    async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        info!("copying maildir messages {id} from folder {from_folder} to folder {to_folder}");

        let ctx = self.ctx.lock().await;
        let from_mdir = ctx.get_maildir_from_folder_alias(from_folder)?;
        let to_mdir = ctx.get_maildir_from_folder_alias(to_folder)?;

        id.iter()
            .filter_map(|id| from_mdir.find(id).ok().flatten())
            .try_for_each(|entry| {
                entry.copy(&to_mdir).map_err(|err| {
                    Error::CopyMessagesMaildirError(
                        err,
                        from_folder.to_owned(),
                        to_folder.to_owned(),
                        entry.path().to_owned(),
                    )
                })?;
                AnyResult::Ok(())
            })?;

        Ok(())
    }
}
