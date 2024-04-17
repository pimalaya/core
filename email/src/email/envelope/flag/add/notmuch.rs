use async_trait::async_trait;

use super::{AddFlags, Flags};
use crate::{
    debug, email::error::Error, envelope::Id, folder::FolderKind, info,
    notmuch::NotmuchContextSync, AnyResult,
};

#[derive(Clone)]
pub struct AddNotmuchFlags {
    ctx: NotmuchContextSync,
}

impl AddNotmuchFlags {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn AddFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn AddFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddFlags for AddNotmuchFlags {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        info!("adding notmuch flag(s) {flags} to envelope {id} from folder {folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let folder_query = if FolderKind::matches_inbox(folder) {
            "folder:\"\"".to_owned()
        } else {
            let folder = config.get_folder_alias(folder);
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
            for flag in flags.iter() {
                msg.add_tag(&flag.to_string())
                    .map_err(Error::NotMuchFailure)?;
            }
        }

        db.close().map_err(Error::NotMuchFailure)?;

        Ok(())
    }
}
