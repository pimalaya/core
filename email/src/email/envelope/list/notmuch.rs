use async_trait::async_trait;
use log::{debug, info, trace};
use thiserror::Error;

use crate::{
    folder::FolderKind, notmuch::NotmuchContextSync, search_query::SearchEmailsQuery, Result,
};

use super::{Envelopes, ListEnvelopes, ListEnvelopesOptions};

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
    async fn list_envelopes(&self, folder: &str, opts: ListEnvelopesOptions) -> Result<Envelopes> {
        info!("listing notmuch envelopes from folder {folder}");

        let ctx = self.ctx.lock().await;
        let config = &ctx.account_config;
        let db = ctx.open_db()?;

        let mut final_query = if FolderKind::matches_inbox(folder) {
            String::from("folder:\"\"")
        } else {
            let folder = config.get_folder_alias(folder.as_ref());
            format!("folder:{folder:?}")
        };

        if let Some(query) = opts.query {
            final_query.push_str(" and ");
            final_query.push_str(&query.to_notmuch_search_query());
        }

        let query_builder = db.create_query(&final_query)?;

        let mut envelopes = Envelopes::from_notmuch_msgs(query_builder.search_messages()?);
        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        let envelopes_len = envelopes.len();
        debug!("found {envelopes_len} notmuch envelopes matching query {final_query}");
        trace!("{envelopes:#?}");

        let page_begin = opts.page * opts.page_size;

        if page_begin > envelopes.len() {
            return Err(Error::GetEnvelopesOutOfBoundsError(
                folder.to_owned(),
                page_begin + 1,
            ))?;
        }

        let page_end = envelopes.len().min(if opts.page_size == 0 {
            envelopes.len()
        } else {
            page_begin + opts.page_size
        });

        *envelopes = envelopes[page_begin..page_end].into();

        db.close()?;

        Ok(envelopes)
    }
}

impl SearchEmailsQuery {
    pub fn to_notmuch_search_query(&self) -> String {
        match self {
            SearchEmailsQuery::And(left, right) => {
                let left = left.to_notmuch_search_query();
                let right = right.to_notmuch_search_query();
                format!("({left}) and ({right})")
            }
            SearchEmailsQuery::Or(left, right) => {
                let left = left.to_notmuch_search_query();
                let right = right.to_notmuch_search_query();
                format!("({left}) or ({right})")
            }
            SearchEmailsQuery::Not(filter) => {
                let filter = filter.to_notmuch_search_query();
                format!("not ({filter})")
            }
            SearchEmailsQuery::Before(date) => {
                let date = date.timestamp();
                format!("date:..@{date}")
            }
            SearchEmailsQuery::After(date) => {
                let date = date.timestamp();
                format!("date:@{date}..")
            }
            SearchEmailsQuery::From(addr) => {
                format!("from:{addr:?}")
            }
            SearchEmailsQuery::To(addr) => {
                format!("to:{addr:?}")
            }
            SearchEmailsQuery::Subject(subject) => {
                format!("subject:{subject:?}")
            }
            SearchEmailsQuery::Body(body) => {
                format!("body:{body:?}")
            }
            SearchEmailsQuery::Keyword(keyword) => {
                format!("keyword:{keyword:?}")
            }
        }
    }
}
