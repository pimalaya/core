use std::collections::HashSet;

use async_trait::async_trait;
use tracing::info;

use super::{Flags, SetFlags};
use crate::{email::error::Error, envelope::Id, maildir::MaildirContextSync, AnyResult};

#[derive(Clone)]
pub struct SetMaildirFlags {
    ctx: MaildirContextSync,
}

impl SetMaildirFlags {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn SetFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn SetFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl SetFlags for SetMaildirFlags {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        info!("setting maildir flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        id.iter()
            .filter_map(|id| mdir.find(id).ok().flatten())
            .try_for_each(|mut entry| {
                entry.update_flags(HashSet::from(flags)).map_err(|err| {
                    Error::SetFlagsMaildirError(
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
