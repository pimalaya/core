use async_trait::async_trait;
use log::{debug, info};

use crate::{envelope::Id, notmuch::NotmuchContextSync, Result};

use super::{AddFlags, Flags};

#[derive(Clone)]
pub struct AddNotmuchFlags {
    ctx: NotmuchContextSync,
}

impl AddNotmuchFlags {
    pub fn new(ctx: impl Into<NotmuchContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<NotmuchContextSync>) -> Box<dyn AddFlags> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl AddFlags for AddNotmuchFlags {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("adding notmuch flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let query = format!("mid:\"/^({})$/\"", id.join("|"));
        debug!("notmuch query: {query:?}");

        let query_builder = db.create_query(&query)?;
        let msgs = query_builder.search_messages()?;

        for msg in msgs {
            for flag in flags.iter() {
                msg.add_tag(&flag.to_string())?;
            }
        }

        db.close()?;

        Ok(())
    }
}
