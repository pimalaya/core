use async_trait::async_trait;
use tracing::{debug, info};

use super::RemoveMessages;
use crate::{
    email::error::Error, envelope::Id, folder::FolderKind, notmuch::NotmuchContextSync, AnyResult,
};

#[derive(Clone)]
pub struct RemoveNotmuchMessages {
    ctx: NotmuchContextSync,
}

impl RemoveNotmuchMessages {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn RemoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn RemoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveMessages for RemoveNotmuchMessages {
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        info!("removing notmuch message(s) {id} from folder {folder}");

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
            let Some(filename) = msg.filenames().find(|f| f.is_file()) else {
                let id = msg.id();
                debug!(?id, "skipping notmuch message with invalid filename");

                continue;
            };

            db.remove_message(filename).map_err(|err| {
                Error::RemoveNotmuchMessageError(err, folder.to_owned(), id.clone())
            })?
        }

        db.close().map_err(Error::NotMuchFailure)?;

        Ok(())
    }
}
