use async_trait::async_trait;
use tracing::info;

use super::RemoveMessages;
use crate::{email::error::Error, envelope::Id, maildir::MaildirContextSync, AnyResult};

#[derive(Clone)]
pub struct RemoveMaildirMessages {
    ctx: MaildirContextSync,
}

impl RemoveMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn RemoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn RemoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveMessages for RemoveMaildirMessages {
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        info!("removing maildir message(s) {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        id.iter()
            .filter_map(|id| mdir.find(id).ok().flatten())
            .try_for_each(|entry| {
                entry.remove().map_err(|err| {
                    Error::RemoveMaildirMessageError(err, folder.to_owned(), id.to_string())
                })
            })?;

        Ok(())
    }
}
