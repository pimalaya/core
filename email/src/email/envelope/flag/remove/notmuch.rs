use async_trait::async_trait;
use log::{debug, info};

use crate::{envelope::Id, notmuch::NotmuchContextSync, Result};

use super::{Flags, RemoveFlags};

#[derive(Clone)]
pub struct RemoveNotmuchFlags {
    ctx: NotmuchContextSync,
}

impl RemoveNotmuchFlags {
    pub fn new(ctx: impl Into<NotmuchContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<NotmuchContextSync>) -> Box<dyn RemoveFlags> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl RemoveFlags for RemoveNotmuchFlags {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("removing notmuch flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let query = format!("mid:\"/^({})$/\"", id.join("|"));
        debug!("notmuch query: {query:?}");

        let query_builder = db.create_query(&query)?;
        let msgs = query_builder.search_messages()?;

        for msg in msgs {
            for flag in flags.iter() {
                msg.remove_tag(&flag.to_string())?;
            }
        }

        db.close()?;

        Ok(())
    }
}
