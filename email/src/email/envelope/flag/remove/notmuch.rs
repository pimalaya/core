use async_trait::async_trait;
use maildirs::MaildirEntry;
use tracing::{debug, info};

use super::{Flags, RemoveFlags};
use crate::{
    email::error::Error, envelope::Id, flag::Flag, folder::FolderKind, notmuch::NotmuchContextSync,
    AnyResult,
};

#[derive(Clone)]
pub struct RemoveNotmuchFlags {
    ctx: NotmuchContextSync,
}

impl RemoveNotmuchFlags {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn RemoveFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn RemoveFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveFlags for RemoveNotmuchFlags {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        info!("removing notmuch flag(s) {flags} to envelope {id} from folder {folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;
        let db = ctx.open_db()?;

        let ref folder = config.get_folder_alias(folder);
        let folder_query = if ctx.maildirpp() && FolderKind::matches_inbox(folder) {
            String::from("folder:\"\"")
        } else {
            format!("folder:{folder:?}")
        };
        let mid_query = format!("mid:\"/^({})$/\"", id.join("|"));
        let query = [folder_query, mid_query].join(" and ");
        debug!("notmuch query: {query:?}");

        let query_builder = db.create_query(&query).map_err(Error::NotMuchFailure)?;
        let msgs = query_builder
            .search_messages()
            .map_err(Error::NotMuchFailure)?;

        for mut msg in msgs {
            let mut entry = MaildirEntry::new(msg.filename());

            for flag in flags.iter() {
                match flag {
                    Flag::Seen => {
                        msg.add_tag("unread").map_err(Error::NotMuchFailure)?;
                        entry
                            .remove_flag(maildirs::Flag::Seen)
                            .map_err(Error::MaildirppFailure)?;
                    }
                    Flag::Answered => {
                        msg.remove_tag("replied").map_err(Error::NotMuchFailure)?;
                        entry
                            .remove_flag(maildirs::Flag::Replied)
                            .map_err(Error::MaildirppFailure)?;
                    }
                    Flag::Flagged => {
                        msg.remove_tag("flagged").map_err(Error::NotMuchFailure)?;
                        entry
                            .remove_flag(maildirs::Flag::Flagged)
                            .map_err(Error::MaildirppFailure)?;
                    }
                    Flag::Deleted => {
                        msg.remove_tag("deleted").map_err(Error::NotMuchFailure)?;
                        entry
                            .remove_flag(maildirs::Flag::Trashed)
                            .map_err(Error::MaildirppFailure)?;
                    }
                    Flag::Draft => {
                        msg.remove_tag("draft").map_err(Error::NotMuchFailure)?;
                        entry
                            .remove_flag(maildirs::Flag::Draft)
                            .map_err(Error::MaildirppFailure)?;
                    }
                    Flag::Custom(tag) => {
                        msg.remove_tag(tag).map_err(Error::NotMuchFailure)?;
                    }
                }

                if msg.filename() != entry.path() {
                    msg = db
                        .index_file(entry.path(), None)
                        .map_err(Error::NotMuchFailure)?;
                }
            }
        }

        db.close().map_err(Error::NotMuchFailure)?;

        Ok(())
    }
}
