use async_trait::async_trait;
use log::info;

use crate::{email::error::Error, envelope::Id, maildir::MaildirContextSync};

use super::CopyMessages;

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
    async fn copy_messages(
        &self,
        from_folder: &str,
        to_folder: &str,
        id: &Id,
    ) -> crate::Result<()> {
        info!("copying maildir messages {id} from folder {from_folder} to folder {to_folder}");

        let ctx = self.ctx.lock().await;
        let from_mdir = ctx.get_maildir_from_folder_name(from_folder)?;
        let to_mdir = ctx.get_maildir_from_folder_name(to_folder)?;

        id.iter().try_for_each(|id| {
            from_mdir.copy_to(id, &to_mdir).map_err(|err| {
                Error::CopyMessagesMaildirError(
                    err,
                    from_folder.to_owned(),
                    to_folder.to_owned(),
                    id.to_owned(),
                )
            })
        })?;

        Ok(())
    }
}
