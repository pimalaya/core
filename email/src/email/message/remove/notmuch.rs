use async_trait::async_trait;
use log::{debug, info};

use crate::{email::error::Error, envelope::Id, folder::FolderKind, notmuch::NotmuchContextSync};

use super::RemoveMessages;

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
    async fn remove_messages(&self, folder: &str, id: &Id) -> crate::Result<()> {
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

        let query_builder = db.create_query(&query)?;
        let msgs = query_builder.search_messages()?;

        for msg in msgs {
            db.remove_message(msg.filename()).map_err(|err| {
                Error::RemoveNotmuchMessageError(err, folder.to_owned(), id.clone())
            })?
        }

        db.close()?;

        Ok(())
    }
}
