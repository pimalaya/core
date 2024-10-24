use std::collections::HashSet;

use async_trait::async_trait;
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::info;

use super::{AddMessage, Flags};
use crate::{
    email::error::Error, envelope::SingleId, flag::Flag, notmuch::NotmuchContextSync, AnyResult,
};

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
    ) -> AnyResult<SingleId> {
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

        let mdir = mdir_ctx.get_maildir_from_folder_alias(&folder)?;
        let mut entry = mdir
            .write_cur(msg, HashSet::from(flags))
            .map_err(Error::MaildirppFailure)?;
        let mut msg = db
            .index_file(entry.path(), None)
            .map_err(Error::NotMuchFailure)?;

        for flag in flags.iter() {
            match flag {
                Flag::Seen => {
                    msg.remove_tag("unread").map_err(Error::NotMuchFailure)?;
                    entry
                        .insert_flag(maildirs::Flag::Seen)
                        .map_err(Error::MaildirppFailure)?;
                }
                Flag::Answered => {
                    msg.add_tag("replied").map_err(Error::NotMuchFailure)?;
                    entry
                        .insert_flag(maildirs::Flag::Replied)
                        .map_err(Error::MaildirppFailure)?;
                }
                Flag::Flagged => {
                    msg.add_tag("flagged").map_err(Error::NotMuchFailure)?;
                    entry
                        .insert_flag(maildirs::Flag::Flagged)
                        .map_err(Error::MaildirppFailure)?;
                }
                Flag::Deleted => {
                    msg.add_tag("deleted").map_err(Error::NotMuchFailure)?;
                    entry
                        .insert_flag(maildirs::Flag::Trashed)
                        .map_err(Error::MaildirppFailure)?;
                }
                Flag::Draft => {
                    msg.add_tag("draft").map_err(Error::NotMuchFailure)?;
                    entry
                        .insert_flag(maildirs::Flag::Draft)
                        .map_err(Error::MaildirppFailure)?;
                }
                Flag::Custom(tag) => {
                    msg.add_tag(tag).map_err(Error::NotMuchFailure)?;
                }
            }

            msg = db
                .index_file(entry.path(), None)
                .map_err(Error::NotMuchFailure)?;
        }

        let id = SingleId::from(msg.id());

        db.close().map_err(Error::NotMuchFailure)?;

        Ok(id)
    }
}
