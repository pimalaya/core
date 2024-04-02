use async_trait::async_trait;
use chrono::TimeDelta;
use imap::extensions::sort::{SortCharset, SortCriterion};
use log::{debug, info, trace};
use std::{collections::HashMap, result};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    email::error::Error,
    envelope::Envelope,
    imap::ImapContextSync,
    search_query::{
        filter::SearchEmailsFilterQuery,
        sort::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        SearchEmailsQuery,
    },
};

use super::{Envelopes, ListEnvelopes, ListEnvelopesOptions};

/// The IMAP query needed to retrieve everything we need to build an
/// [envelope]: UID, flags and headers (Message-ID, From, To, Subject,
/// Date).
pub const LIST_ENVELOPES_QUERY: &str = "(UID FLAGS ENVELOPE)";

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
    ) -> crate::Result<Envelopes> {
        info!("listing imap envelopes from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let folder_size = ctx
            .exec(
                |session| session.select(&folder_encoded),
                |err| Error::SelectFolderImapError(err, folder.clone()).into(),
            )
            .await?
            .exists as usize;
        debug!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let envelopes = if let Some(query) = opts.query {
            let filters = query.to_imap_sort_query();
            let sorters = query.to_imap_sort_criteria();

            let mut uids = ctx
                .exec(
                    |session| session.uid_sort(&sorters, SortCharset::Utf8, &filters),
                    |err| {
                        Error::SearchEnvelopesImapError(err, folder.clone(), filters.clone()).into()
                    },
                )
                .await?;

            apply_pagination(&mut uids, opts.page, opts.page_size)?;

            let range = uids.iter().fold(String::new(), |mut range, uid| {
                if !range.is_empty() {
                    range.push(',');
                }
                range.push_str(&uid.to_string());
                range
            });

            let fetches = ctx
                .exec(
                    |session| session.uid_fetch(&range, LIST_ENVELOPES_QUERY),
                    |err| Error::ListEnvelopesImapError(err, folder.clone(), range.clone()).into(),
                )
                .await?;

            let mut envelopes: HashMap<String, Envelope> =
                HashMap::from_iter(fetches.iter().filter_map(
                    |fetch| match Envelope::from_imap_fetch(fetch) {
                        Ok(envelope) => Some((envelope.id.clone(), envelope)),
                        Err(err) => {
                            debug!("cannot build imap envelope, skipping it: {err}");
                            None
                        }
                    },
                ));

            uids.into_iter()
                .map(|uid| envelopes.remove_entry(&uid.to_string()).unwrap().1)
                .collect()
        } else {
            let range = build_page_range(opts.page, opts.page_size, folder_size)?;

            let fetches = ctx
                .exec(
                    |session| session.fetch(&range, LIST_ENVELOPES_QUERY),
                    |err| Error::ListEnvelopesImapError(err, folder.clone(), range.clone()).into(),
                )
                .await?;

            let mut envelopes = Envelopes::from_imap_fetches(fetches);
            envelopes.sort_by(|a, b| b.date.cmp(&a.date));
            envelopes
        };

        debug!("found {} imap envelopes", envelopes.len());
        trace!("{envelopes:#?}");

        Ok(envelopes)
    }
}

impl SearchEmailsQuery {
    pub fn to_imap_sort_query(&self) -> String {
        let query = self
            .filter
            .as_ref()
            .map(|f| f.to_imap_sort_query())
            .unwrap_or_default();
        let query = query.trim();
        let query = if query.is_empty() {
            String::from("ALL")
        } else {
            query.to_owned()
        };
        query
    }

    pub fn to_imap_sort_criteria(&self) -> Vec<SortCriterion> {
        let criteria: Vec<SortCriterion> = self
            .sort
            .as_ref()
            .map(|sorters| {
                sorters
                    .iter()
                    .map(|sorter| sorter.to_imap_sort_criterion())
                    .collect()
            })
            .unwrap_or_default();

        if criteria.is_empty() {
            vec![SortCriterion::Reverse(&SortCriterion::Date)]
        } else {
            criteria
        }
    }
}

impl SearchEmailsFilterQuery {
    pub fn to_imap_sort_query(&self) -> String {
        match self {
            SearchEmailsFilterQuery::And(left, right) => {
                let left = left.to_imap_sort_query();
                let right = right.to_imap_sort_query();
                format!("{left} {right}")
            }
            SearchEmailsFilterQuery::Or(left, right) => {
                let left = left.to_imap_sort_query();
                let right = right.to_imap_sort_query();
                format!("OR ({left}) ({right})")
            }
            SearchEmailsFilterQuery::Not(filter) => {
                let filter = filter.to_imap_sort_query();
                format!("NOT ({filter})")
            }
            SearchEmailsFilterQuery::Date(date) => {
                format!("SENTON {}", date.format("%d-%b-%Y"))
            }
            SearchEmailsFilterQuery::BeforeDate(date) => {
                format!("SENTBEFORE {}", date.format("%d-%b-%Y"))
            }
            SearchEmailsFilterQuery::AfterDate(date) => {
                // imap sentsince is inclusive, so we add one day to
                // the date filter.
                let date = *date + TimeDelta::try_days(1).unwrap();
                format!("SENTSINCE {}", date.format("%d-%b-%Y"))
            }
            SearchEmailsFilterQuery::From(pattern) => {
                format!("FROM {pattern}")
            }
            SearchEmailsFilterQuery::To(pattern) => {
                format!("TO {pattern}")
            }
            SearchEmailsFilterQuery::Subject(pattern) => {
                format!("SUBJECT {pattern}")
            }
            SearchEmailsFilterQuery::Body(pattern) => {
                format!("BODY {pattern}")
            }
            SearchEmailsFilterQuery::Flag(flag) => {
                let flag = flag.to_imap_query_string();
                format!("KEYWORD {flag}")
            }
        }
    }
}

impl SearchEmailsSorter {
    pub fn to_imap_sort_criterion(&self) -> SortCriterion {
        use SearchEmailsSorterKind::*;
        use SearchEmailsSorterOrder::*;
        use SortCriterion::Reverse;

        match self {
            SearchEmailsSorter(Date, Ascending) => SortCriterion::Date,
            SearchEmailsSorter(Date, Descending) => Reverse(&SortCriterion::Date),
            SearchEmailsSorter(From, Ascending) => SortCriterion::From,
            SearchEmailsSorter(From, Descending) => Reverse(&SortCriterion::From),
            SearchEmailsSorter(To, Ascending) => SortCriterion::To,
            SearchEmailsSorter(To, Descending) => SortCriterion::Reverse(&SortCriterion::To),
            SearchEmailsSorter(Subject, Ascending) => SortCriterion::Subject,
            SearchEmailsSorter(Subject, Descending) => Reverse(&SortCriterion::Subject),
        }
    }
}

fn apply_pagination(
    uids: &mut Vec<u32>,
    page: usize,
    page_size: usize,
) -> result::Result<(), Error> {
    let total = uids.len();
    let page_cursor = page * page_size;
    if page_cursor >= total {
        Err(Error::BuildPageRangeOutOfBoundsImapError(page + 1))?
    }

    if page_size == 0 {
        return Ok(());
    }

    let page_size = page_size.min(total);
    *uids = uids[0..page_size].into();
    Ok(())
}

/// Builds the IMAP sequence set for the give page, page size and
/// total size.
fn build_page_range(page: usize, page_size: usize, total: usize) -> result::Result<String, Error> {
    let page_cursor = page * page_size;
    if page_cursor >= total {
        Err(Error::BuildPageRangeOutOfBoundsImapError(page + 1))?
    }

    let range = if page_size == 0 {
        String::from("1:*")
    } else {
        let page_size = page_size.min(total);
        let mut count = 1;
        let mut cursor = total - (total.min(page_cursor));
        let mut range = cursor.to_string();
        while cursor > 1 && count < page_size {
            count += 1;
            cursor -= 1;
            if count > 1 {
                range.push(',');
            }
            range.push_str(&cursor.to_string());
        }
        range
    };

    Ok(range)
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_page_range_out_of_bounds() {
        // page * page_size < size
        assert_eq!(super::build_page_range(0, 5, 5).unwrap(), "5,4,3,2,1");

        // page * page_size = size
        assert!(matches!(
            super::build_page_range(1, 5, 5).unwrap_err(),
            super::Error::BuildPageRangeOutOfBoundsImapError(2),
        ));

        // page * page_size > size
        assert!(matches!(
            super::build_page_range(2, 5, 5).unwrap_err(),
            super::Error::BuildPageRangeOutOfBoundsImapError(3),
        ));
    }

    #[test]
    fn build_page_range_page_size_0() {
        assert_eq!(super::build_page_range(0, 0, 3).unwrap(), "1:*");
        assert_eq!(super::build_page_range(1, 0, 4).unwrap(), "1:*");
        assert_eq!(super::build_page_range(2, 0, 5).unwrap(), "1:*");
    }

    #[test]
    fn build_page_range_page_size_smaller_than_size() {
        assert_eq!(super::build_page_range(0, 3, 5).unwrap(), "5,4,3");
        assert_eq!(super::build_page_range(1, 3, 5).unwrap(), "2,1");
        assert_eq!(super::build_page_range(1, 4, 5).unwrap(), "1");
    }

    #[test]
    fn build_page_range_page_bigger_than_size() {
        assert_eq!(super::build_page_range(0, 10, 5).unwrap(), "5,4,3,2,1");
    }
}
