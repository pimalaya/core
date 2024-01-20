use async_trait::async_trait;
use log::info;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{envelope::SingleId, notmuch::NotmuchContextSync, Result};

use super::{AddMessage, Flags};

static EXTRACT_FOLDER_FROM_QUERY: Lazy<Regex> =
    Lazy::new(|| Regex::new("folder:\"?([^\"]*)\"?").unwrap());

#[derive(Clone)]
pub struct AddNotmuchMessage {
    ctx: NotmuchContextSync,
}

impl AddNotmuchMessage {
    pub fn new(ctx: impl Into<NotmuchContextSync>) -> Self {
        Self { ctx: ctx.into() }
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
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        info!("adding notmuch message to folder {folder} with flags {flags}");

        let ctx = self.ctx.lock().await;
        let config = &ctx.account_config;
        let db = ctx.open_db()?;

        // FIXME
        //
        // let folder_alias = config.find_folder_alias(folder);
        // let folder = match folder_alias {
        //     Some(ref alias) => EXTRACT_FOLDER_FROM_QUERY
        //         .captures(alias)
        //         .map(|m| m[1].to_owned())
        //         .unwrap_or_else(|| folder.to_owned()),
        //     None => folder.to_owned(),
        // };
        // let path = ctx.session.path().join(folder);
        // let mdir = Maildir::from(
        //     path.canonicalize()
        //         .map_err(|err| Error::CanonicalizePathError(err, path.clone()))?,
        // );
        // let mdir_internal_id = mdir
        //     .store_cur_with_flags(email, &flags.to_mdir_string())
        //     .map_err(|err| Error::StoreWithFlagsError(err, mdir.path().to_owned()))?;
        // trace!("added email internal maildir id: {mdir_internal_id}");

        // let entry = mdir
        //     .find(&mdir_internal_id)
        //     .ok_or(Error::FindMaildirEmailById)?;
        // let path = entry
        //     .path()
        //     .canonicalize()
        //     .map_err(|err| Error::CanonicalizePathError(err, entry.path().clone()))?;
        // trace!("path: {path:?}");

        // let email = db.index_file(&path, None).map_err(Error::IndexFileError)?;

        db.close()?;

        Ok(SingleId::from("0"))
    }
}
