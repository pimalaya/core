use std::collections::HashSet;

use async_trait::async_trait;
use tracing::info;

use super::{Flags, RemoveFlags};
use crate::{email::error::Error, envelope::Id, maildir::MaildirContextSync, AnyResult};

#[derive(Clone)]
pub struct RemoveMaildirFlags {
    ctx: MaildirContextSync,
}

impl RemoveMaildirFlags {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn RemoveFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn RemoveFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveFlags for RemoveMaildirFlags {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        info!("removing maildir flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        id.iter()
            .filter_map(|id| mdir.find(id).ok().flatten())
            .try_for_each(|mut entry| {
                entry.remove_flags(HashSet::from(flags)).map_err(|err| {
                    Error::RemoveFlagsMaildirError(
                        err,
                        folder.to_owned(),
                        id.to_string(),
                        flags.clone(),
                    )
                })
            })?;

        Ok(())
    }
}
