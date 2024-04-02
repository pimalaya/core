use async_trait::async_trait;
use log::info;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{envelope::SingleId, notmuch::NotmuchContextSync};

use super::{AddMessage, Flags};

static EXTRACT_FOLDER_FROM_QUERY: Lazy<Regex> =
    Lazy::new(|| Regex::new("folder:\"?([^\"]*)\"?").unwrap());

#[derive(Clone)]
pub struct AddNotmuchMessage {
    ctx: NotmuchContextSync,
}

impl AddNotmuchMessage {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn AddMessage> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn AddMessage>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddMessage for AddNotmuchMessage {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        msg: &[u8],
        flags: &Flags,
    ) -> crate::Result<SingleId> {
        info!("adding notmuch message to folder {folder} with flags {flags}");

        let ctx = self.ctx.lock().await;
        let mdir_ctx = &ctx.mdir_ctx;
        let db = ctx.open_db()?;

        let folder_alias = &self.ctx.account_config.find_folder_alias(folder);
        let folder = match folder_alias {
            Some(ref alias) => EXTRACT_FOLDER_FROM_QUERY
                .captures(alias)
                .map(|m| m[1].to_owned())
                .unwrap_or(folder.to_owned()),
            None => folder.to_owned(),
        };

        let mdir = mdir_ctx.get_maildir_from_folder_name(&folder)?;
        let id = mdir.store_cur_with_flags(msg, &flags.to_mdir_string())?;
        let msg = mdir.find(&id).unwrap();

        let msg = db.index_file(msg.path(), None)?;

        flags
            .iter()
            .try_for_each(|flag| msg.add_tag(&flag.to_string()))?;

        let id = SingleId::from(msg.id());

        db.close()?;

        Ok(id)
    }
}
