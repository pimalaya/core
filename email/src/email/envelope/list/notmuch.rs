use async_trait::async_trait;
use log::{debug, info, trace};
use thiserror::Error;

use crate::{folder::FolderKind, notmuch::NotmuchContextSync, Result};

use super::{Envelopes, ListEnvelopes};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot list notmuch envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsError(String, usize),
}

#[derive(Clone)]
pub struct ListNotmuchEnvelopes {
    ctx: NotmuchContextSync,
}

impl ListNotmuchEnvelopes {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn ListEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn ListEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListEnvelopes for ListNotmuchEnvelopes {
    async fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        info!("listing notmuch envelopes from folder {folder}");

        let ctx = self.ctx.lock().await;
        let config = &ctx.account_config;
        let db = ctx.open_db()?;

        let query = if FolderKind::matches_inbox(folder) {
            String::from("folder:\"\"")
        } else {
            let folder = config.get_folder_alias(folder.as_ref());
            format!("folder:{folder:?}")
        };

        let query_builder = db.create_query(&query)?;

        let mut envelopes = Envelopes::from_notmuch_msgs(query_builder.search_messages()?);
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        let envelopes_len = envelopes.len();
        debug!("found {envelopes_len} notmuch envelopes matching query {query}");
        trace!("{envelopes:#?}");

        let page_begin = page * page_size;

        if page_begin > envelopes.len() {
            return Err(Error::GetEnvelopesOutOfBoundsError(
                folder.to_owned(),
                page_begin + 1,
            ))?;
        }

        let page_end = envelopes.len().min(if page_size == 0 {
            envelopes.len()
        } else {
            page_begin + page_size
        });

        *envelopes = envelopes[page_begin..page_end].into();

        db.close()?;

        Ok(envelopes)
    }
}
