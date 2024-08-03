use std::{num::NonZeroU32, result};

use async_trait::async_trait;
use chrono::TimeDelta;
use imap_next::imap_types::{
    core::Vec1,
    extensions::sort::{SortCriterion, SortKey},
    search::SearchKey,
    sequence::{SeqOrUid, Sequence},
};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{Envelopes, ListEnvelopes, ListEnvelopesOptions};
use crate::{
    debug,
    email::error::Error,
    imap::ImapContextSync,
    info,
    search_query::{
        filter::SearchEmailsFilterQuery,
        sort::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        SearchEmailsQuery,
    },
    trace, AnyResult, Result,
};

#[derive(Clone, Debug)]
pub struct ListImapEnvelopes {
    ctx: ImapContextSync,
}

impl ListImapEnvelopes {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn ListEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ListEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListEnvelopes for ListImapEnvelopes {
    async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<Envelopes> {
        info!("listing imap envelopes from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let folder_size = ctx.select_mailbox(folder_encoded).await?.exists.unwrap() as usize;
        debug!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let envelopes = if let Some(query) = opts.query.as_ref() {
            let search_criteria = query.to_imap_search_criteria();
            let sort_criteria = query.to_imap_sort_criteria();

            let mut envelopes = ctx
                .sort_envelopes(sort_criteria, search_criteria)
                .await
                .unwrap();

            apply_pagination(&mut envelopes, opts.page, opts.page_size)?;

            envelopes
        } else {
            let seq = build_sequence(opts.page, opts.page_size, folder_size)?;
            let mut envelopes = ctx.fetch_envelopes_by_sequence(seq.into()).await?;
            envelopes.sort_by(|a, b| b.date.cmp(&a.date));
            envelopes
        };

        debug!("found {} imap envelopes", envelopes.len());
        trace!("{envelopes:#?}");

        Ok(envelopes)
    }
}

impl SearchEmailsQuery {
    pub fn to_imap_search_criteria(&self) -> Vec1<SearchKey<'static>> {
        self.filter
            .as_ref()
            .map(|f| f.to_imap_search_criterion())
            .unwrap_or(SearchKey::All)
            .into()
    }

    pub fn to_imap_sort_criteria(&self) -> Vec1<SortCriterion> {
        let criteria: Vec<_> = self
            .sort
            .as_ref()
            .map(|sorters| {
                sorters
                    .iter()
                    .map(|sorter| sorter.to_imap_sort_criterion())
                    .collect()
            })
            .unwrap_or_default();

        Vec1::try_from(criteria).unwrap_or_else(|_| {
            Vec1::from(SortCriterion {
                reverse: true,
                key: SortKey::Date,
            })
        })
    }
}

impl SearchEmailsFilterQuery {
    pub fn to_imap_search_criterion(&self) -> SearchKey<'static> {
        match self {
            SearchEmailsFilterQuery::And(left, right) => {
                let criteria = vec![
                    left.to_imap_search_criterion(),
                    right.to_imap_search_criterion(),
                ];
                SearchKey::And(criteria.try_into().unwrap())
            }
            SearchEmailsFilterQuery::Or(left, right) => {
                let left = left.to_imap_search_criterion();
                let right = right.to_imap_search_criterion();
                SearchKey::Or(Box::new(left), Box::new(right))
            }
            SearchEmailsFilterQuery::Not(filter) => {
                let criterion = filter.to_imap_search_criterion();
                SearchKey::Not(Box::new(criterion))
            }
            SearchEmailsFilterQuery::Date(date) => SearchKey::SentOn((*date).try_into().unwrap()),
            SearchEmailsFilterQuery::BeforeDate(date) => {
                SearchKey::SentBefore((*date).try_into().unwrap())
            }
            SearchEmailsFilterQuery::AfterDate(date) => {
                // imap sentsince is inclusive, so we add one day to
                // the date filter.
                let date = *date + TimeDelta::try_days(1).unwrap();
                SearchKey::SentSince(date.try_into().unwrap())
            }
            SearchEmailsFilterQuery::From(pattern) => {
                SearchKey::From(pattern.clone().try_into().unwrap())
            }
            SearchEmailsFilterQuery::To(pattern) => {
                SearchKey::To(pattern.clone().try_into().unwrap())
            }
            SearchEmailsFilterQuery::Subject(pattern) => {
                SearchKey::Subject(pattern.clone().try_into().unwrap())
            }
            SearchEmailsFilterQuery::Body(pattern) => {
                SearchKey::Body(pattern.clone().try_into().unwrap())
            }
            SearchEmailsFilterQuery::Flag(flag) => flag.clone().try_into().unwrap(),
        }
    }
}

impl SearchEmailsSorter {
    pub fn to_imap_sort_criterion(&self) -> SortCriterion {
        use SearchEmailsSorterKind::*;
        use SearchEmailsSorterOrder::*;

        match self {
            SearchEmailsSorter(Date, Ascending) => SortCriterion {
                reverse: false,
                key: SortKey::Date,
            },
            SearchEmailsSorter(Date, Descending) => SortCriterion {
                reverse: true,
                key: SortKey::Date,
            },
            SearchEmailsSorter(From, Ascending) => SortCriterion {
                reverse: false,
                key: SortKey::From,
            },
            SearchEmailsSorter(From, Descending) => SortCriterion {
                reverse: true,
                key: SortKey::From,
            },
            SearchEmailsSorter(To, Ascending) => SortCriterion {
                reverse: false,
                key: SortKey::To,
            },
            SearchEmailsSorter(To, Descending) => SortCriterion {
                reverse: true,
                key: SortKey::To,
            },
            SearchEmailsSorter(Subject, Ascending) => SortCriterion {
                reverse: false,
                key: SortKey::Subject,
            },
            SearchEmailsSorter(Subject, Descending) => SortCriterion {
                reverse: true,
                key: SortKey::Subject,
            },
        }
    }
}

fn apply_pagination(
    envelopes: &mut Envelopes,
    page: usize,
    page_size: usize,
) -> result::Result<(), Error> {
    let total = envelopes.len();
    let page_cursor = page * page_size;
    if page_cursor >= total {
        Err(Error::BuildPageRangeOutOfBoundsImapError(page + 1))?
    }

    if page_size == 0 {
        return Ok(());
    }

    let page_size = page_size.min(total);
    *envelopes = Envelopes(envelopes[0..page_size].to_vec());
    Ok(())
}

/// Builds the IMAP sequence set for the give page, page size and
/// total size.
fn build_sequence(page: usize, page_size: usize, total: usize) -> Result<Sequence> {
    let seq = if page_size == 0 {
        Sequence::Single(SeqOrUid::Asterisk)
    } else {
        let page_cursor = page * page_size;
        if page_cursor >= total {
            Err(Error::BuildPageRangeOutOfBoundsImapError(page + 1))?
        }

        let mut count = 1;
        let mut cursor = total - (total.min(page_cursor));

        let page_size = page_size.min(total);
        let from = SeqOrUid::Value(NonZeroU32::new(cursor as u32).unwrap());
        while cursor > 1 && count < page_size {
            count += 1;
            cursor -= 1;
        }
        let to = SeqOrUid::Value(NonZeroU32::new(cursor as u32).unwrap());
        Sequence::Range(from, to)
    };

    Ok(seq)
}
