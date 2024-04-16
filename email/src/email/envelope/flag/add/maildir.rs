use crate::info;
use async_trait::async_trait;

use crate::{email::error::Error, envelope::Id, maildir::MaildirContextSync, AnyResult};

use super::{AddFlags, Flags};

#[derive(Clone)]
pub struct AddMaildirFlags {
    ctx: MaildirContextSync,
}

impl AddMaildirFlags {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn AddFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn AddFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddFlags for AddMaildirFlags {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        info!("adding maildir flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.add_flags(id, &flags.to_mdir_string()).map_err(|err| {
                Error::AddFlagsMaildirError(err, folder.to_owned(), id.to_string(), flags.clone())
            })
        })?;

        Ok(())
    }
}
