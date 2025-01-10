use std::{collections::HashMap, num::NonZeroU32, result};

use async_trait::async_trait;
use chrono::TimeDelta;
use futures::{stream::FuturesUnordered, StreamExt};
use imap_client::imap_next::imap_types::{
    core::Vec1,
    extensions::sort::{SortCriterion, SortKey},
    search::SearchKey,
    sequence::{SeqOrUid, Sequence, SequenceSet},
};
use tracing::{debug, info, instrument, trace};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{Envelopes, ListEnvelopes, ListEnvelopesOptions};
use crate::{
    email::error::Error,
    envelope::Envelope,
    imap,
    imap::ImapContext,
    search_query::{
        filter::SearchEmailsFilterQuery,
        sort::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        SearchEmailsQuery,
    },
    AnyResult, Result,
};

static MAX_SEQUENCE_SIZE: u8 = u8::MAX; // 255

#[derive(Clone, Debug)]
pub struct ListImapEnvelopes {
    ctx: ImapContext,
}

impl ListImapEnvelopes {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn ListEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn ListEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListEnvelopes for ListImapEnvelopes {
    #[instrument(skip(self), level = "trace")]
    async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<Envelopes> {
        info!("listing IMAP envelopes from mailbox {folder}");

        let config = &self.ctx.account_config;
        let mut client = self.ctx.client().await;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!(name = folder_encoded, "UTF7-encoded mailbox");

        let data = client.select_mailbox(folder_encoded.clone()).await?;
        let folder_size = data.exists.unwrap_or_default() as usize;
        debug!(name = folder_encoded, ?data, "mailbox selected");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let envelopes = if let Some(query) = opts.query.as_ref() {
            let sort_supported = client.ext_sort_supported();
            let sort_criteria = query.to_imap_sort_criteria();
            let search_criteria = query.to_imap_search_criteria();

            let uids = if sort_supported {
                client
                    .sort_uids(sort_criteria.clone(), search_criteria.clone())
                    .await
            } else {
                client.search_uids(search_criteria.clone()).await
            }?;

            // this client is not used anymore, so we can drop it now
            // in order to free one client slot from the clients
            // connection pool
            drop(client);

            if uids.is_empty() {
                return Ok(Envelopes::default());
            }

            // if the SORT extension is supported by the client,
            // envelopes can be paginated straight away
            let uids = if sort_supported {
                paginate(&uids, opts.page, opts.page_size)?
            } else {
                &uids
            };

            let uids_chunks = uids.chunks(MAX_SEQUENCE_SIZE as usize);
            let uids_chunks_len = uids_chunks.len();

            debug!(?uids, "fetching envelopes using {uids_chunks_len} chunks");

            let mut fetches = FuturesUnordered::from_iter(uids_chunks.map(|uids| {
                let ctx = self.ctx.clone();
                let mbox = folder_encoded.clone();
                let uids = SequenceSet::try_from(uids.to_vec()).unwrap();

                tokio::spawn(async move {
                    let mut client = ctx.client().await;
                    client.select_mailbox(mbox).await?;
                    client.fetch_envelopes(uids).await
                })
            }))
            .enumerate()
            .fold(
                Ok(HashMap::<String, Envelope>::default()),
                |all_envelopes, (n, envelopes)| async move {
                    let Ok(mut all_envelopes) = all_envelopes else {
                        return all_envelopes;
                    };

                    match envelopes {
                        Err(err) => {
                            return Err(imap::Error::JoinClientError(err));
                        }
                        Ok(Err(err)) => {
                            return Err(err);
                        }
                        Ok(Ok(envelopes)) => {
                            debug!("fetched envelopes chunk {}/{uids_chunks_len}", n + 1);

                            for envelope in envelopes {
                                all_envelopes.insert(envelope.id.clone(), envelope);
                            }

                            Ok(all_envelopes)
                        }
                    }
                },
            )
            .await?;

            let mut envelopes: Envelopes = uids
                .iter()
                .flat_map(|uid| fetches.remove(&uid.to_string()))
                .collect();

            // if the SORT extension is NOT supported by the client,
            // envelopes are sorted and paginated only now
            if !sort_supported {
                opts.sort_envelopes(&mut envelopes);
                apply_pagination(&mut envelopes, opts.page, opts.page_size)?;
            }

            envelopes
        } else {
            let seq = build_sequence(opts.page, opts.page_size, folder_size)?;
            let mut envelopes = client.fetch_envelopes_by_sequence(seq.into()).await?;
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

fn paginate<T>(items: &[T], page: usize, page_size: usize) -> Result<&[T]> {
    if page_size == 0 {
        return Ok(items);
    }

    let total = items.len();
    let page_cursor = page * page_size;
    if page_cursor >= total {
        Err(Error::BuildPageRangeOutOfBoundsImapError(page + 1))?
    }

    Ok(&items[0..page_size.min(total)])
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
        Sequence::Range(SeqOrUid::try_from(1).unwrap(), SeqOrUid::Asterisk)
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
