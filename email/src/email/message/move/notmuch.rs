use async_trait::async_trait;
use log::{debug, info};

use crate::{email::error::Error, envelope::Id, folder::FolderKind, notmuch::NotmuchContextSync};

use super::MoveMessages;

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
    async fn move_messages(
        &self,
        from_folder: &str,
        to_folder: &str,
        id: &Id,
    ) -> crate::Result<()> {
        info!("moving notmuch messages {id} from folder {from_folder} to folder {to_folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;

        let mdir_ctx = &ctx.mdir_ctx;
        let mdir_from = mdir_ctx.get_maildir_from_folder_name(from_folder)?;
        let mdir_to = mdir_ctx.get_maildir_from_folder_name(to_folder)?;

        let db = ctx.open_db()?;

        let folder_query = if FolderKind::matches_inbox(from_folder) {
            "folder:\"\"".to_owned()
        } else {
            let folder = config.get_folder_alias(from_folder);
            format!("folder:{folder:?}")
        };
        let mid_query = format!("mid:\"/^({})$/\"", id.join("|"));
        let query = [folder_query, mid_query].join(" and ");
        let query_builder = db.create_query(&query).map_err(Error::NotMuchFailure)?;
        let msgs = query_builder
            .search_messages()
            .map_err(Error::NotMuchFailure)?;

        for msg in msgs {
            let mdir_id = mdir_from.list_cur().find_map(|entry| {
                let entry = entry.ok()?;
                if entry.path() == msg.filename() {
                    Some(entry.id().to_owned())
                } else {
                    None
                }
            });

            match &mdir_id {
                None => {
                    let path = msg.filename().to_string_lossy();
                    debug!("cannot move missing notmuch message {path}");
                    break;
                }
                Some(mdir_id) => {
                    mdir_from
                        .move_to(mdir_id, &mdir_to)
                        .map_err(Error::MaildirppFailure)?;
                    msg.reindex(db.default_indexopts().map_err(Error::NotMuchFailure)?)
                        .map_err(Error::NotMuchFailure)?;
                    let mdir_entry = mdir_to.find(mdir_id).unwrap();
                    db.index_file(mdir_entry.path(), None)
                        .map_err(Error::NotMuchFailure)?;
                }
            }
        }

        Ok(())
    }
}
