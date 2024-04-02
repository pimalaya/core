use async_trait::async_trait;
use log::info;

use crate::{folder::error::Error, maildir::MaildirContextSync};

use super::ExpungeFolder;

pub struct ExpungeMaildirFolder {
    ctx: MaildirContextSync,
}

impl ExpungeMaildirFolder {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn ExpungeFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ExpungeFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeMaildirFolder {
    async fn expunge_folder(&self, folder: &str) -> crate::Result<()> {
        info!("expunging maildir folder {folder}");

        let ctx = self.ctx.lock().await;

        let mdir = ctx.get_maildir_from_folder_name(folder)?;
        let entries = mdir
            .list_cur()
            .collect::<maildirpp::Result<Vec<_>>>()
            .map_err(|err| Error::ListCurrentFolderMaildirError(err, mdir.path().to_owned()))?;
        entries
            .iter()
            .filter_map(|entry| {
                if entry.is_trashed() {
                    Some(entry.id())
                } else {
                    None
                }
            })
            .try_for_each(|id| {
                mdir.delete(id).map_err(|err| {
                    Error::DeleteMessageMaildirError(err, mdir.path().to_owned(), id.to_owned())
                })
            })?;

        Ok(())
    }
}
