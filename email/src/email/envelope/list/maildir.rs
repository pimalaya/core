use std::{fs, path::Path};

use async_trait::async_trait;
use mail_parser::MessageParser;
use tracing::{debug, info, trace, warn};

use super::{Envelopes, ListEnvelopes, ListEnvelopesOptions};
use crate::{
    email::error::Error,
    envelope::Envelope,
    maildir::MaildirContextSync,
    search_query::{filter::SearchEmailsFilterQuery, SearchEmailsQuery},
    AnyResult,
};

#[cfg(test)]
static USER_TZ: &chrono::Utc = &chrono::Utc;
#[cfg(not(test))]
static USER_TZ: &chrono::Local = &chrono::Local;

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
    async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<Envelopes> {
        info!("listing maildir envelopes from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entries = mdir.read().map_err(Error::ListMaildirEntriesError)?;
        let mut envelopes = Envelopes::from_mdir_entries(entries, opts.query.as_ref());
        debug!("found {} maildir envelopes", envelopes.len());
        trace!("{envelopes:#?}");

        let page_begin = opts.page * opts.page_size;
        debug!("page begin: {}", page_begin);
        if page_begin > envelopes.len() {
            return Err(Error::GetEnvelopesOutOfBoundsMaildirError(
                folder.to_owned(),
                page_begin + 1,
            )
            .into());
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
        self.filter
            .as_ref()
            .map(|f| f.matches_maildir_search_query(envelope, msg_path))
            .unwrap_or(true)
    }
}

fn contains_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    for window in haystack.windows(needle.len()) {
        if window.eq_ignore_ascii_case(needle) {
            return true;
        }
    }

    false
}

impl SearchEmailsFilterQuery {
    pub fn matches_maildir_search_query(&self, envelope: &Envelope, msg_path: &Path) -> bool {
        match self {
            SearchEmailsFilterQuery::And(left, right) => {
                let left = left.matches_maildir_search_query(envelope, msg_path);
                let right = right.matches_maildir_search_query(envelope, msg_path);
                left && right
            }
            SearchEmailsFilterQuery::Or(left, right) => {
                let left = left.matches_maildir_search_query(envelope, msg_path);
                let right = right.matches_maildir_search_query(envelope, msg_path);
                left || right
            }
            SearchEmailsFilterQuery::Not(filter) => {
                !filter.matches_maildir_search_query(envelope, msg_path)
            }
            SearchEmailsFilterQuery::Date(date) => {
                &envelope.date.with_timezone(USER_TZ).date_naive() == date
            }
            SearchEmailsFilterQuery::BeforeDate(date) => {
                &envelope.date.with_timezone(USER_TZ).date_naive() < date
            }
            SearchEmailsFilterQuery::AfterDate(date) => {
                &envelope.date.with_timezone(USER_TZ).date_naive() > date
            }
            SearchEmailsFilterQuery::From(pattern) => {
                let pattern = pattern.as_bytes();
                if let Some(name) = &envelope.from.name {
                    if contains_ignore_ascii_case(name.as_bytes(), pattern) {
                        return true;
                    }
                }
                contains_ignore_ascii_case(envelope.from.addr.as_bytes(), pattern)
            }
            SearchEmailsFilterQuery::To(pattern) => {
                let pattern = pattern.as_bytes();
                if let Some(name) = &envelope.to.name {
                    if contains_ignore_ascii_case(name.as_bytes(), pattern) {
                        return true;
                    }
                }
                contains_ignore_ascii_case(envelope.to.addr.as_bytes(), pattern)
            }
            SearchEmailsFilterQuery::Subject(pattern) => {
                contains_ignore_ascii_case(envelope.subject.as_bytes(), pattern.as_bytes())
            }
            SearchEmailsFilterQuery::Body(pattern) => match fs::read(msg_path) {
                Ok(contents) => {
                    if let Some(msg) = MessageParser::new().parse(&contents) {
                        for plain in msg.text_bodies() {
                            if contains_ignore_ascii_case(plain.contents(), pattern.as_bytes()) {
                                return true;
                            }
                        }
                        for html in msg.html_bodies() {
                            if contains_ignore_ascii_case(html.contents(), pattern.as_bytes()) {
                                return true;
                            }
                        }
                    }
                    false
                }
                Err(_err) => {
                    warn!("cannot find message at {msg_path:?}, skipping body filter");
                    trace!("{_err:?}");
                    true
                }
            },
            SearchEmailsFilterQuery::Flag(flag) => envelope.flags.contains(flag),
        }
    }
}
