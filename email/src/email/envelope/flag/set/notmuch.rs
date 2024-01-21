use async_trait::async_trait;
use log::{debug, info};

use crate::{envelope::Id, folder::FolderKind, notmuch::NotmuchContextSync, Result};

use super::{Flags, SetFlags};

#[derive(Clone)]
pub struct SetNotmuchFlags {
    ctx: NotmuchContextSync,
}

impl SetNotmuchFlags {
    pub fn new(ctx: impl Into<NotmuchContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<NotmuchContextSync>) -> Box<dyn SetFlags> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl SetFlags for SetNotmuchFlags {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("setting notmuch flag(s) {flags} to envelope {id} from folder {folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let folder_query = if FolderKind::matches_inbox(folder) {
            format!("folder:\"\"")
        } else {
            let folder = config.get_folder_alias(folder.as_ref());
            format!("folder:{folder:?}")
        };
        let mid_query = format!("mid:\"/^({})$/\"", id.join("|"));
        let query = [folder_query, mid_query].join(" and ");
        debug!("notmuch query: {query:?}");

        let query_builder = db.create_query(&query)?;
        let msgs = query_builder.search_messages()?;

        for msg in msgs {
            msg.remove_all_tags()?;

            for flag in flags.iter() {
                msg.add_tag(&flag.to_string())?;
            }
        }

        db.close()?;

        Ok(())
    }
}
