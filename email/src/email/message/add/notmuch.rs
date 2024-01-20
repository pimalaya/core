use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    envelope::SingleId,
    maildir::{config::MaildirConfig, MaildirContext, MaildirContextSync},
    notmuch::NotmuchContextSync,
    Result,
};

use super::{AddMessage, Flags};

static EXTRACT_FOLDER_FROM_QUERY: Lazy<Regex> =
    Lazy::new(|| Regex::new("folder:\"?([^\"]*)\"?").unwrap());

#[derive(Clone)]
pub struct AddNotmuchMessage {
    ctx: NotmuchContextSync,
    mdir_ctx: MaildirContextSync,
}

impl AddNotmuchMessage {
    pub fn new(ctx: impl Into<NotmuchContextSync>) -> Self {
        let ctx = ctx.into();
        let root = Maildir::from(ctx.notmuch_config.get_maildir_path().to_owned());

        let maildir_config = MaildirConfig {
            root_dir: root.path().to_owned(),
        };

        let mdir_ctx = MaildirContext {
            account_config: ctx.account_config.clone(),
            maildir_config,
            root,
        };

        Self {
            ctx,
            mdir_ctx: mdir_ctx.into(),
        }
    }

    pub fn new_boxed(ctx: impl Into<NotmuchContextSync>) -> Box<dyn AddMessage> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl AddMessage for AddNotmuchMessage {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        info!("adding notmuch message to folder {folder} with flags {flags}");

        let folder_alias = &self.ctx.account_config.find_folder_alias(folder);
        let folder = match folder_alias {
            Some(ref alias) => EXTRACT_FOLDER_FROM_QUERY
                .captures(alias)
                .map(|m| m[1].to_owned())
                .unwrap_or(folder.to_owned()),
            None => folder.to_owned(),
        };

        let msg = {
            let ctx = self.mdir_ctx.lock().await;
            let mdir = ctx.get_maildir_from_folder_name(&folder)?;
            let id = mdir.store_cur_with_flags(msg, &flags.to_mdir_string())?;
            mdir.find(&id).unwrap()
        };

        let id = {
            let ctx = self.ctx.lock().await;
            let db = ctx.open_db()?;
            let msg = db.index_file(msg.path(), None)?;
            db.close()?;
            SingleId::from(msg.id())
        };

        Ok(id)
    }
}
