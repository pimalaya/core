use async_trait::async_trait;
use maildirs::MaildirEntry;
use tracing::{debug, info};

use super::MoveMessages;
use crate::{
    email::error::Error, envelope::Id, folder::FolderKind, notmuch::NotmuchContextSync, AnyResult,
};

#[derive(Clone)]
pub struct MoveNotmuchMessages {
    pub(crate) ctx: NotmuchContextSync,
}

impl MoveNotmuchMessages {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn MoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn MoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl MoveMessages for MoveNotmuchMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        info!("moving notmuch messages {id} from folder {from_folder} to folder {to_folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;

        let mdir_ctx = &ctx.mdir_ctx;
        let mdir_to = mdir_ctx.get_maildir_from_folder_alias(to_folder)?;

        let db = ctx.open_db()?;

        let ref from_folder = config.get_folder_alias(from_folder);
        let folder_query = if ctx.maildirpp() && FolderKind::matches_inbox(from_folder) {
            String::from("folder:\"\"")
        } else {
            format!("folder:{from_folder:?}")
        };
        let mid_query = format!("mid:\"/^({})$/\"", id.join("|"));
        let query = [folder_query, mid_query].join(" and ");
        let query_builder = db.create_query(&query).map_err(Error::NotMuchFailure)?;
        let msgs = query_builder
            .search_messages()
            .map_err(Error::NotMuchFailure)?;

        for msg in msgs {
            let Some(filename) = msg.filenames().find(|f| f.is_file()) else {
                let id = msg.id();
                debug!(?id, "skipping notmuch message with invalid filename");

                continue;
            };

            let entry = MaildirEntry::new(filename);
            let path = entry.r#move(&mdir_to).map_err(Error::MaildirppFailure)?;

            if let Some(path) = path {
                msg.reindex(db.default_indexopts().map_err(Error::NotMuchFailure)?)
                    .map_err(Error::NotMuchFailure)?;
                db.index_file(path, None).map_err(Error::NotMuchFailure)?;
            }
        }

        Ok(())
    }
}
