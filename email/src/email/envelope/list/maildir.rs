use async_trait::async_trait;
use chumsky::container::Seq;
use log::{debug, info, trace, warn};
use std::{fs, path::Path};
use thiserror::Error;

use crate::{
    envelope::Envelope,
    maildir::MaildirContextSync,
    search_query::{filter::SearchEmailsQueryFilter, SearchEmailsQuery},
    Result,
};

use super::{Envelopes, ListEnvelopes, ListEnvelopesOptions};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot list maildir envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsError(String, usize),
}

#[derive(Clone)]
pub struct ListMaildirEnvelopes {
    ctx: MaildirContextSync,
}

impl ListMaildirEnvelopes {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn ListEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ListEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListEnvelopes for ListMaildirEnvelopes {
    async fn list_envelopes(&self, folder: &str, opts: ListEnvelopesOptions) -> Result<Envelopes> {
        info!("listing maildir envelopes from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        let mut envelopes = Envelopes::from_mdir_entries(mdir.list_cur(), opts.query.as_ref());
        debug!("found {} maildir envelopes", envelopes.len());
        trace!("{envelopes:#?}");

        let page_begin = opts.page * opts.page_size;
        debug!("page begin: {}", page_begin);
        if page_begin > envelopes.len() {
            return Err(
                Error::GetEnvelopesOutOfBoundsError(folder.to_owned(), page_begin + 1).into(),
            );
        }

        let page_end = envelopes.len().min(if opts.page_size == 0 {
            envelopes.len()
        } else {
            page_begin + opts.page_size
        });
        debug!("page end: {}", page_end);

        opts.sort_envelopes(&mut envelopes);
        *envelopes = envelopes[page_begin..page_end].into();

        Ok(envelopes)
    }
}

impl SearchEmailsQuery {
    pub fn matches_maildir_search_query(&self, envelope: &Envelope, msg_path: &Path) -> bool {
        self.filters
            .as_ref()
            .map(|f| f.matches_maildir_search_query(envelope, msg_path))
            .unwrap_or(true)
    }
}

impl SearchEmailsQueryFilter {
    pub fn matches_maildir_search_query(&self, envelope: &Envelope, msg_path: &Path) -> bool {
        match self {
            SearchEmailsQueryFilter::And(left, right) => {
                let left = left.matches_maildir_search_query(envelope, msg_path);
                let right = right.matches_maildir_search_query(envelope, msg_path);
                left && right
            }
            SearchEmailsQueryFilter::Or(left, right) => {
                let left = left.matches_maildir_search_query(envelope, msg_path);
                let right = right.matches_maildir_search_query(envelope, msg_path);
                left || right
            }
            SearchEmailsQueryFilter::Not(filter) => {
                !filter.matches_maildir_search_query(envelope, msg_path)
            }
            SearchEmailsQueryFilter::Date(date) => &envelope.date <= date,
            SearchEmailsQueryFilter::BeforeDate(date) => &envelope.date <= date,
            SearchEmailsQueryFilter::AfterDate(date) => &envelope.date > date,
            SearchEmailsQueryFilter::From(pattern) => {
                if let Some(name) = &envelope.from.name {
                    if name.contains(pattern) {
                        return true;
                    }
                }
                envelope.from.addr.contains(pattern)
            }
            SearchEmailsQueryFilter::To(pattern) => {
                if let Some(name) = &envelope.to.name {
                    if name.contains(pattern) {
                        return true;
                    }
                }
                envelope.to.addr.contains(pattern)
            }
            SearchEmailsQueryFilter::Subject(pattern) => envelope.subject.contains(pattern),
            SearchEmailsQueryFilter::Body(pattern) => match fs::read(msg_path) {
                Ok(contents) => contents
                    .windows(pattern.as_bytes().len())
                    .find(|window| window.eq_ignore_ascii_case(pattern.as_bytes()))
                    .is_some(),
                Err(err) => {
                    warn!("cannot find message at {msg_path:?}, skipping body filter");
                    trace!("{err:?}");
                    true
                }
            },
            SearchEmailsQueryFilter::Keyword(pattern) => {
                for flag in envelope.flags.iter() {
                    if flag.to_string().contains(pattern) {
                        return true;
                    }
                }
                false
            }
        }
    }
}
