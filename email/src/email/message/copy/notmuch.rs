use async_trait::async_trait;
use log::info;
use std::fs;

use crate::{
    email::error::Error,
    envelope::Id,
    flag::{Flag, Flags},
    folder::FolderKind,
    notmuch::NotmuchContextSync,
    AnyResult,
};

use super::CopyMessages;

#[derive(Clone)]
pub struct CopyNotmuchMessages {
    ctx: NotmuchContextSync,
}

impl CopyNotmuchMessages {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn CopyMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn CopyMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CopyMessages for CopyNotmuchMessages {
    async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        info!("copying notmuch messages {id} from folder {from_folder} to folder {to_folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;

        let mdir_ctx = &ctx.mdir_ctx;
        let mdir = mdir_ctx.get_maildir_from_folder_name(to_folder)?;

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
            let flags = Flags::from_iter([Flag::Seen]).to_mdir_string();
            let content = fs::read(msg.filename()).map_err(Error::FileReadFailure)?;
            let mdir_id = mdir
                .store_cur_with_flags(&content, &flags)
                .map_err(Error::MaildirppFailure)?;
            let mdir_entry = mdir.find(&mdir_id).unwrap();
            db.index_file(mdir_entry.path(), None)
                .map_err(Error::NotMuchFailure)?;
        }

        Ok(())
    }
}
