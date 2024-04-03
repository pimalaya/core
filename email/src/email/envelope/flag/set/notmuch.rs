use async_trait::async_trait;
use log::{debug, info};

use crate::{email::error::Error, envelope::Id, folder::FolderKind, notmuch::NotmuchContextSync};

use super::{Flags, SetFlags};

#[derive(Clone)]
pub struct SetNotmuchFlags {
    ctx: NotmuchContextSync,
}

impl SetNotmuchFlags {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn SetFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn SetFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl SetFlags for SetNotmuchFlags {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> crate::Result<()> {
        info!("setting notmuch flag(s) {flags} to envelope {id} from folder {folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let folder_query = if FolderKind::matches_inbox(folder) {
            "folder:\"\"".to_owned()
        } else {
            let folder = config.get_folder_alias(folder.as_ref());
            format!("folder:{folder:?}")
        };
        let mid_query = format!("mid:\"/^({})$/\"", id.join("|"));
        let query = [folder_query, mid_query].join(" and ");
        debug!("notmuch query: {query:?}");

        let query_builder = db.create_query(&query).map_err(Error::NotMuchFailure)?;
        let msgs = query_builder
            .search_messages()
            .map_err(Error::NotMuchFailure)?;

        for msg in msgs {
            msg.remove_all_tags().map_err(Error::NotMuchFailure)?;

            for flag in flags.iter() {
                msg.add_tag(&flag.to_string())
                    .map_err(Error::NotMuchFailure)?;
            }
        }

        db.close().map_err(Error::NotMuchFailure)?;

        Ok(())
    }
}
