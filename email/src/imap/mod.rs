pub mod config;
mod error;

use std::{
    collections::HashMap, env, fmt, io::ErrorKind::ConnectionReset, num::NonZeroU32, sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use imap_client::{
    client::tokio::{Client, ClientError},
    imap_next::imap_types::{
        auth::AuthMechanism,
        core::{IString, NString, Vec1},
        extensions::{
            sort::SortCriterion,
            thread::{Thread, ThreadingAlgorithm},
        },
        fetch::MessageDataItem,
        flag::{Flag, StoreType},
        search::SearchKey,
        sequence::SequenceSet,
    },
    stream::Error as StreamError,
    tasks::{tasks::select::SelectDataUnvalidated, SchedulerError},
};
use once_cell::sync::Lazy;
use tokio::{
    select,
    sync::{oneshot, Mutex, MutexGuard},
    time::sleep,
};
use tracing::{debug, instrument, trace, warn};

use self::config::{ImapAuthConfig, ImapConfig};
#[doc(inline)]
pub use self::error::{Error, Result};
#[cfg(feature = "oauth2")]
use crate::account::config::oauth2::OAuth2Method;
#[cfg(feature = "thread")]
use crate::envelope::thread::{imap::ThreadImapEnvelopes, ThreadEnvelopes};
#[cfg(feature = "watch")]
use crate::envelope::watch::{imap::WatchImapEnvelopes, WatchEnvelopes};
use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    envelope::{
        get::{imap::GetImapEnvelope, GetEnvelope},
        imap::FETCH_ENVELOPES,
        list::{imap::ListImapEnvelopes, ListEnvelopes},
        Envelope, Envelopes,
    },
    flag::{
        add::{imap::AddImapFlags, AddFlags},
        remove::{imap::RemoveImapFlags, RemoveFlags},
        set::{imap::SetImapFlags, SetFlags},
    },
    folder::{
        add::{imap::AddImapFolder, AddFolder},
        delete::{imap::DeleteImapFolder, DeleteFolder},
        expunge::{imap::ExpungeImapFolder, ExpungeFolder},
        list::{imap::ListImapFolders, ListFolders},
        purge::{imap::PurgeImapFolder, PurgeFolder},
        Folders,
    },
    message::{
        add::{imap::AddImapMessage, AddMessage},
        copy::{imap::CopyImapMessages, CopyMessages},
        delete::{imap::DeleteImapMessages, DeleteMessages},
        get::{imap::GetImapMessages, GetMessages},
        imap::{FETCH_MESSAGES, PEEK_MESSAGES},
        peek::{imap::PeekImapMessages, PeekMessages},
        r#move::{imap::MoveImapMessages, MoveMessages},
        remove::{imap::RemoveImapMessages, RemoveMessages},
        Messages,
    },
    retry::{self, Retry, RetryState},
    tls::{Encryption, Tls, TlsProvider},
    AnyResult,
};

static ID_PARAMS: Lazy<Vec<(IString<'static>, NString<'static>)>> = Lazy::new(|| {
    vec![
        (
            "name".try_into().unwrap(),
            NString(
                env::var("CARGO_PKG_NAME")
                    .ok()
                    .map(|e| e.try_into().unwrap()),
            ),
        ),
        (
            "vendor".try_into().unwrap(),
            NString(
                env::var("CARGO_PKG_NAME")
                    .ok()
                    .map(|e| e.try_into().unwrap()),
            ),
        ),
        (
            "version".try_into().unwrap(),
            NString(
                env::var("CARGO_PKG_VERSION")
                    .ok()
                    .map(|e| e.try_into().unwrap()),
            ),
        ),
        (
            "support-url".try_into().unwrap(),
            NString(Some(
                "https://github.com/orgs/pimalaya/discussions/new?category=q-a"
                    .try_into()
                    .unwrap(),
            )),
        ),
    ]
});

enum ImapRetryState<T> {
    Retry,
    TimedOut,
    Ok(std::result::Result<T, ClientError>),
}

/// The IMAP backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`ImapContextSync`].
pub struct ImapClient {
    pub id: u8,

    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The next gen IMAP client builder.
    pub client_builder: ImapClientBuilder,

    /// The next gen IMAP client.
    inner: Client,

    /// The selected mailbox.
    mailbox: Option<String>,

    retry: Retry,
}

impl ImapClient {
    async fn retry<T>(
        &mut self,
        res: retry::Result<std::result::Result<T, ClientError>>,
    ) -> Result<ImapRetryState<T>> {
        match self.retry.next(res) {
            RetryState::Retry => {
                debug!(attempt = self.retry.attempts, "request timed out");
                Ok(ImapRetryState::Retry)
            }
            RetryState::TimedOut => {
                return Ok(ImapRetryState::TimedOut);
            }
            RetryState::Ok(Err(ClientError::Stream(err))) => {
                match err {
                    StreamError::State(SchedulerError::UnexpectedByeResponse(bye)) => {
                        debug!(reason = bye.text.to_string(), "stream closed");
                    }
                    StreamError::Io(err) if err.kind() == ConnectionReset => {
                        debug!("connection reset");
                    }
                    StreamError::Closed => {
                        debug!("stream closed");
                    }
                    StreamError::Io(err) if err.kind() == ConnectionReset => {
                        debug!("connection reset");
                    }
                    err => {
                        let err = ClientError::Stream(err);
                        return Ok(ImapRetryState::Ok(Err(err)));
                    }
                };

                debug!("re-connecting…");

                self.inner = self.client_builder.build().await?;

                if let Some(mbox) = &self.mailbox {
                    self.inner
                        .select(mbox.clone())
                        .await
                        .map_err(Error::SelectMailboxError)?;
                }

                self.retry.attempts = 0;
                Ok(ImapRetryState::Retry)
            }
            RetryState::Ok(res) => {
                return Ok(ImapRetryState::Ok(res));
            }
        }
    }

    pub fn ext_sort_supported(&self) -> bool {
        self.inner.state.ext_sort_supported()
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn noop(&mut self) -> Result<()> {
        self.retry.reset();

        loop {
            let res = self.retry.timeout(self.inner.noop()).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::NoOpTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::NoOpError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn select_mailbox(&mut self, mbox: impl ToString) -> Result<SelectDataUnvalidated> {
        self.retry.reset();

        let data = loop {
            let res = self
                .retry
                .timeout(self.inner.select(mbox.to_string()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::SelectMailboxTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::SelectMailboxError),
            }
        }?;

        self.mailbox = Some(mbox.to_string());

        Ok(data)
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn examine_mailbox(&mut self, mbox: impl ToString) -> Result<SelectDataUnvalidated> {
        self.retry.reset();

        loop {
            let res = self
                .retry
                .timeout(self.inner.examine(mbox.to_string()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::ExamineMailboxTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::ExamineMailboxError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn create_mailbox(&mut self, mbox: impl ToString) -> Result<()> {
        self.retry.reset();

        loop {
            let res = self
                .retry
                .timeout(self.inner.create(mbox.to_string()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::CreateMailboxTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::CreateMailboxError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn list_all_mailboxes(&mut self, config: &AccountConfig) -> Result<Folders> {
        self.retry.reset();

        let mboxes = loop {
            let res = self.retry.timeout(self.inner.list("", "*")).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::ListMailboxesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::ListMailboxesError),
            }
        }?;

        let folders = Folders::from_imap_mailboxes(config, mboxes);

        Ok(folders)
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn expunge_mailbox(&mut self, mbox: impl ToString) -> Result<usize> {
        self.select_mailbox(mbox).await?;

        self.retry.reset();

        let expunged = loop {
            let res = self.retry.timeout(self.inner.expunge()).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::ExpungeMailboxTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::ExpungeMailboxError),
            }
        }?;

        Ok(expunged.len())
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn purge_mailbox(&mut self, mbox: impl ToString) -> Result<usize> {
        self.select_mailbox(mbox).await?;

        self.add_deleted_flag_silently("1:*".try_into().unwrap())
            .await?;

        let expunged = loop {
            let res = self.retry.timeout(self.inner.expunge()).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::ExpungeMailboxTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::ExpungeMailboxError),
            }
        }?;

        Ok(expunged.len())
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn delete_mailbox(&mut self, mbox: impl ToString) -> Result<()> {
        self.retry.reset();

        loop {
            let res = self
                .retry
                .timeout(self.inner.delete(mbox.to_string()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::DeleteMailboxTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::DeleteMailboxError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn fetch_envelopes(&mut self, uids: SequenceSet) -> Result<Envelopes> {
        self.retry.reset();

        let fetches = loop {
            let res = self
                .retry
                .timeout(self.inner.uid_fetch(uids.clone(), FETCH_ENVELOPES.clone()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        Ok(Envelopes::from_imap_data_items(fetches))
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn fetch_envelopes_map(
        &mut self,
        uids: SequenceSet,
    ) -> Result<HashMap<String, Envelope>> {
        let fetches = loop {
            let res = self
                .retry
                .timeout(self.inner.uid_fetch(uids.clone(), FETCH_ENVELOPES.clone()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        let map = fetches
            .into_values()
            .map(|items| {
                let envelope = Envelope::from_imap_data_items(items.as_ref());
                (envelope.id.clone(), envelope)
            })
            .collect();

        Ok(map)
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn fetch_first_envelope(&mut self, uid: u32) -> Result<Envelope> {
        let items = loop {
            let task = self
                .inner
                .uid_fetch_first(uid.try_into().unwrap(), FETCH_ENVELOPES.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        Ok(Envelope::from_imap_data_items(items.as_ref()))
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn fetch_envelopes_by_sequence(&mut self, seq: SequenceSet) -> Result<Envelopes> {
        let fetches = loop {
            let res = self
                .retry
                .timeout(self.inner.fetch(seq.clone(), FETCH_ENVELOPES.clone()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        Ok(Envelopes::from_imap_data_items(fetches))
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn fetch_all_envelopes(&mut self) -> Result<Envelopes> {
        self.fetch_envelopes_by_sequence("1:*".try_into().unwrap())
            .await
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn sort_uids(
        &mut self,
        sort_criteria: impl IntoIterator<Item = SortCriterion> + Clone,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Vec<NonZeroU32>> {
        loop {
            let task = self
                .inner
                .uid_sort(sort_criteria.clone(), search_criteria.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::SortUidsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::SortUidsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn search_uids(
        &mut self,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Vec<NonZeroU32>> {
        loop {
            let res = self
                .retry
                .timeout(self.inner.uid_search(search_criteria.clone()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::SearchUidsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::SearchUidsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn sort_envelopes(
        &mut self,
        sort_criteria: impl IntoIterator<Item = SortCriterion> + Clone,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Envelopes> {
        let fetches = loop {
            let task = self.inner.uid_sort_or_fallback(
                sort_criteria.clone(),
                search_criteria.clone(),
                FETCH_ENVELOPES.clone(),
            );

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        Ok(Envelopes::from(fetches))
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn thread_envelopes(
        &mut self,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Vec<Thread>> {
        loop {
            let task = self
                .inner
                .uid_thread(ThreadingAlgorithm::References, search_criteria.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::ThreadMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::ThreadMessagesError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn idle(
        &mut self,
        wait_for_shutdown_request: &mut oneshot::Receiver<()>,
    ) -> Result<()> {
        let tag = self.inner.enqueue_idle();

        select! {
            output = self.inner.idle(tag.clone()) => {
                output.map_err(Error::StartIdleError)?;
                Ok(())
            },
            _ = wait_for_shutdown_request => {
                debug!("shutdown requested, sending done command…");
                self.inner.idle_done(tag.clone()).await.map_err(Error::StopIdleError)?;
                Err(Error::IdleInterruptedError)
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn add_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        loop {
            let task = self
                .inner
                .uid_store(uids.clone(), StoreType::Add, flags.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn add_deleted_flag(
        &mut self,
        uids: SequenceSet,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        loop {
            let task = self
                .inner
                .uid_store(uids.clone(), StoreType::Add, Some(Flag::Deleted));

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn add_deleted_flag_silently(&mut self, uids: SequenceSet) -> Result<()> {
        loop {
            let task =
                self.inner
                    .uid_silent_store(uids.clone(), StoreType::Add, Some(Flag::Deleted));

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn add_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        loop {
            let task = self
                .inner
                .uid_silent_store(uids.clone(), StoreType::Add, flags.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn set_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        loop {
            let task = self
                .inner
                .uid_store(uids.clone(), StoreType::Replace, flags.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn set_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        loop {
            let task = self
                .inner
                .uid_silent_store(uids.clone(), StoreType::Replace, flags.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn remove_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        loop {
            let task = self
                .inner
                .uid_store(uids.clone(), StoreType::Remove, flags.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn remove_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        loop {
            let task = self
                .inner
                .uid_silent_store(uids.clone(), StoreType::Remove, flags.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::StoreFlagsTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::StoreFlagsError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn add_message(
        &mut self,
        mbox: impl ToString,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
        msg: impl AsRef<[u8]> + Clone,
    ) -> Result<NonZeroU32> {
        let id = loop {
            let task =
                self.inner
                    .appenduid_or_fallback(mbox.to_string(), flags.clone(), msg.clone());

            let res = self.retry.timeout(task).await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::AddMessageTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::AddMessageError),
            }
        }?;

        id.ok_or(Error::FindAppendedMessageUidError)
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn fetch_messages(&mut self, uids: SequenceSet) -> Result<Messages> {
        let mut fetches = loop {
            let res = self
                .retry
                .timeout(self.inner.uid_fetch(uids.clone(), FETCH_MESSAGES.clone()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        let fetches: Vec<_> = uids
            .iter(NonZeroU32::MAX)
            .filter_map(|ref uid| fetches.remove(uid))
            .collect();

        Ok(Messages::from(fetches))
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn peek_messages(&mut self, uids: SequenceSet) -> Result<Messages> {
        let mut fetches = loop {
            let res = self
                .retry
                .timeout(self.inner.uid_fetch(uids.clone(), PEEK_MESSAGES.clone()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::FetchMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::FetchMessagesError),
            }
        }?;

        let fetches: Vec<_> = uids
            .iter(NonZeroU32::MAX)
            .filter_map(|ref uid| fetches.remove(uid))
            .collect();

        Ok(Messages::from(fetches))
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn copy_messages(&mut self, uids: SequenceSet, mbox: impl ToString) -> Result<()> {
        loop {
            let res = self
                .retry
                .timeout(self.inner.uid_copy(uids.clone(), mbox.to_string()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::CopyMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::CopyMessagesError),
            }
        }
    }

    #[instrument(skip_all, fields(client = self.id))]
    pub async fn move_messages(&mut self, uids: SequenceSet, mbox: impl ToString) -> Result<()> {
        loop {
            let res = self
                .retry
                .timeout(self.inner.uid_move(uids.clone(), mbox.to_string()))
                .await;

            match self.retry(res).await? {
                ImapRetryState::Retry => continue,
                ImapRetryState::TimedOut => break Err(Error::MoveMessagesTimedOutError),
                ImapRetryState::Ok(res) => break res.map_err(Error::MoveMessagesError),
            }
        }
    }
}

impl fmt::Debug for ImapClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImapContext")
            .field("id", &self.id)
            .field("mailbox", &self.mailbox)
            .field("imap_config", &self.imap_config)
            .finish_non_exhaustive()
    }
}

/// The sync version of the IMAP backend context.
///
/// This is just an IMAP session wrapped into a mutex, so the same
/// IMAP session can be shared and updated across multiple threads.
#[derive(Debug, Clone)]
pub struct ImapContext {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    clients: Vec<Arc<Mutex<ImapClient>>>,
}

impl ImapContext {
    pub async fn client(&self) -> MutexGuard<'_, ImapClient> {
        loop {
            let lock = self
                .clients
                .iter()
                .find_map(|client| client.try_lock().ok());

            if let Some(ctx) = lock {
                let total = self.clients.len();
                let id = ctx.id;
                debug!("client {id}/{total} is free, locking it");
                break ctx;
            } else {
                trace!("no free client, sleeping for 1s");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

impl BackendContext for ImapContext {}

/// The IMAP backend context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ImapContextBuilder {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The prebuilt IMAP credentials.
    prebuilt_credentials: Option<String>,

    pool_size: u8,
}

impl ImapContextBuilder {
    pub fn new(account_config: Arc<AccountConfig>, imap_config: Arc<ImapConfig>) -> Self {
        let pool_size = imap_config.clients_pool_size();

        Self {
            account_config,
            imap_config,
            prebuilt_credentials: None,
            pool_size,
        }
    }

    pub async fn prebuild_credentials(&mut self) -> Result<()> {
        self.prebuilt_credentials = Some(self.imap_config.build_credentials().await?);
        Ok(())
    }

    pub async fn with_prebuilt_credentials(mut self) -> Result<Self> {
        self.prebuild_credentials().await?;
        Ok(self)
    }

    pub fn with_pool_size(mut self, pool_size: u8) -> Self {
        self.pool_size = pool_size;
        self
    }
}

#[cfg(feature = "sync")]
impl crate::sync::hash::SyncHash for ImapContextBuilder {
    fn sync_hash(&self, state: &mut std::hash::DefaultHasher) {
        self.imap_config.sync_hash(state);
    }
}

#[async_trait]
impl BackendContextBuilder for ImapContextBuilder {
    type Context = ImapContext;

    fn check_up(&self) -> Option<BackendFeature<Self::Context, dyn CheckUp>> {
        Some(Arc::new(CheckUpImap::some_new_boxed))
    }

    fn add_folder(&self) -> Option<BackendFeature<Self::Context, dyn AddFolder>> {
        Some(Arc::new(AddImapFolder::some_new_boxed))
    }

    fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
        Some(Arc::new(ListImapFolders::some_new_boxed))
    }

    fn expunge_folder(&self) -> Option<BackendFeature<Self::Context, dyn ExpungeFolder>> {
        Some(Arc::new(ExpungeImapFolder::some_new_boxed))
    }

    fn purge_folder(&self) -> Option<BackendFeature<Self::Context, dyn PurgeFolder>> {
        Some(Arc::new(PurgeImapFolder::some_new_boxed))
    }

    fn delete_folder(&self) -> Option<BackendFeature<Self::Context, dyn DeleteFolder>> {
        Some(Arc::new(DeleteImapFolder::some_new_boxed))
    }

    fn get_envelope(&self) -> Option<BackendFeature<Self::Context, dyn GetEnvelope>> {
        Some(Arc::new(GetImapEnvelope::some_new_boxed))
    }

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        Some(Arc::new(ListImapEnvelopes::some_new_boxed))
    }

    #[cfg(feature = "thread")]
    fn thread_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ThreadEnvelopes>> {
        Some(Arc::new(ThreadImapEnvelopes::some_new_boxed))
    }

    #[cfg(feature = "watch")]
    fn watch_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn WatchEnvelopes>> {
        Some(Arc::new(WatchImapEnvelopes::some_new_boxed))
    }

    fn add_flags(&self) -> Option<BackendFeature<Self::Context, dyn AddFlags>> {
        Some(Arc::new(AddImapFlags::some_new_boxed))
    }

    fn set_flags(&self) -> Option<BackendFeature<Self::Context, dyn SetFlags>> {
        Some(Arc::new(SetImapFlags::some_new_boxed))
    }

    fn remove_flags(&self) -> Option<BackendFeature<Self::Context, dyn RemoveFlags>> {
        Some(Arc::new(RemoveImapFlags::some_new_boxed))
    }

    fn add_message(&self) -> Option<BackendFeature<Self::Context, dyn AddMessage>> {
        Some(Arc::new(AddImapMessage::some_new_boxed))
    }

    fn peek_messages(&self) -> Option<BackendFeature<Self::Context, dyn PeekMessages>> {
        Some(Arc::new(PeekImapMessages::some_new_boxed))
    }

    fn get_messages(&self) -> Option<BackendFeature<Self::Context, dyn GetMessages>> {
        Some(Arc::new(GetImapMessages::some_new_boxed))
    }

    fn copy_messages(&self) -> Option<BackendFeature<Self::Context, dyn CopyMessages>> {
        Some(Arc::new(CopyImapMessages::some_new_boxed))
    }

    fn move_messages(&self) -> Option<BackendFeature<Self::Context, dyn MoveMessages>> {
        Some(Arc::new(MoveImapMessages::some_new_boxed))
    }

    fn delete_messages(&self) -> Option<BackendFeature<Self::Context, dyn DeleteMessages>> {
        Some(Arc::new(DeleteImapMessages::some_new_boxed))
    }

    fn remove_messages(&self) -> Option<BackendFeature<Self::Context, dyn RemoveMessages>> {
        Some(Arc::new(RemoveImapMessages::some_new_boxed))
    }

    async fn build(self) -> AnyResult<Self::Context> {
        let client_builder =
            ImapClientBuilder::new(self.imap_config.clone(), self.prebuilt_credentials);

        debug!("building {} IMAP clients", self.pool_size);

        let clients = FuturesUnordered::from_iter((0..self.pool_size).map(move |i| {
            let mut client_builder = client_builder.clone();
            tokio::spawn(async move {
                let client = client_builder.build().await?;
                Ok((i + 1, client_builder, client))
            })
        }))
        .map(|res| match res {
            Err(err) => Err(Error::JoinClientError(err)),
            Ok(Err(err)) => Err(Error::BuildClientError(Box::new(err))),
            Ok(Ok((id, client_builder, inner))) => Ok(Arc::new(Mutex::new(ImapClient {
                id,
                account_config: self.account_config.clone(),
                imap_config: self.imap_config.clone(),
                client_builder,
                inner,
                mailbox: Default::default(),
                retry: Default::default(),
            }))),
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<_>>()?;

        Ok(ImapContext {
            account_config: self.account_config,
            imap_config: self.imap_config,
            clients,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CheckUpImap {
    ctx: ImapContext,
}

impl CheckUpImap {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn CheckUp> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn CheckUp>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CheckUp for CheckUpImap {
    #[instrument(skip_all)]
    async fn check_up(&self) -> AnyResult<()> {
        debug!("executing check up backend feature");
        Ok(self.ctx.client().await.noop().await?)
    }
}

#[derive(Clone, Debug)]
pub struct ImapClientBuilder {
    pub config: Arc<ImapConfig>,
    pub credentials: Option<String>,
}

impl ImapClientBuilder {
    pub fn new(config: Arc<ImapConfig>, credentials: Option<String>) -> Self {
        Self {
            config,
            credentials,
        }
    }

    /// Creates a new session from an IMAP configuration and optional
    /// pre-built credentials.
    ///
    /// Pre-built credentials are useful to prevent building them
    /// every time a new session is created. The main use case is for
    /// the synchronization, where multiple sessions can be created in
    /// a row.
    #[instrument(name = "client::build", skip(self))]
    pub async fn build(&mut self) -> Result<Client> {
        let mut client = match &self.config.encryption {
            Some(Encryption::None) => Client::insecure(&self.config.host, self.config.port)
                .await
                .map_err(|err| {
                    let host = self.config.host.clone();
                    let port = self.config.port.clone();
                    Error::BuildInsecureClientError(err, host, port)
                })?,
            Some(Encryption::Tls(Tls {
                provider: Some(TlsProvider::None),
            }))
            | Some(Encryption::StartTls(Tls {
                provider: Some(TlsProvider::None),
            })) => {
                return Err(Error::BuildTlsClientMissingProvider);
            }
            #[cfg(feature = "rustls")]
            Some(Encryption::Tls(Tls {
                provider: Some(TlsProvider::Rustls(_)) | None,
            }))
            | None => Client::rustls(&self.config.host, self.config.port, false)
                .await
                .map_err(|err| {
                    let host = self.config.host.clone();
                    let port = self.config.port.clone();
                    Error::BuildStartTlsClientError(err, host, port)
                })?,
            #[cfg(feature = "native-tls")]
            Some(Encryption::Tls(Tls {
                provider: Some(TlsProvider::NativeTls(_)),
            })) => Client::native_tls(&self.config.host, self.config.port, false)
                .await
                .map_err(|err| {
                    let host = self.config.host.clone();
                    let port = self.config.port.clone();
                    Error::BuildStartTlsClientError(err, host, port)
                })?,
            #[cfg(feature = "rustls")]
            Some(Encryption::StartTls(Tls {
                provider: Some(TlsProvider::Rustls(_)) | None,
            })) => Client::rustls(&self.config.host, self.config.port, true)
                .await
                .map_err(|err| {
                    let host = self.config.host.clone();
                    let port = self.config.port.clone();
                    Error::BuildStartTlsClientError(err, host, port)
                })?,
            #[cfg(feature = "native-tls")]
            Some(Encryption::StartTls(Tls {
                provider: Some(TlsProvider::NativeTls(_)),
            })) => Client::native_tls(&self.config.host, self.config.port, true)
                .await
                .map_err(|err| {
                    let host = self.config.host.clone();
                    let port = self.config.port.clone();
                    Error::BuildStartTlsClientError(err, host, port)
                })?,
        };

        client
            .state
            .set_some_idle_timeout(self.config.find_watch_timeout().map(Duration::from_secs));

        match &self.config.auth {
            ImapAuthConfig::Password(passwd) => {
                debug!("using password authentication");

                let passwd = match self.credentials.as_ref() {
                    Some(passwd) => passwd.to_string(),
                    None => passwd
                        .get()
                        .await
                        .map_err(Error::GetPasswdImapError)?
                        .lines()
                        .next()
                        .ok_or(Error::GetPasswdEmptyImapError)?
                        .to_owned(),
                };

                let mechanisms: Vec<_> =
                    client.state.supported_auth_mechanisms().cloned().collect();
                let mut authenticated = false;

                debug!(?mechanisms, "supported auth mechanisms");

                for mechanism in mechanisms {
                    debug!(?mechanism, "trying auth mechanism…");

                    let auth = match mechanism {
                        AuthMechanism::Plain => {
                            client
                                .authenticate_plain(self.config.login.as_str(), passwd.as_str())
                                .await
                        }
                        // TODO
                        // AuthMechanism::Login => {
                        //     client
                        //         .authenticate_login(self.config.login.as_str(), passwd.as_str())
                        //         .await
                        // }
                        _ => {
                            continue;
                        }
                    };

                    if let Err(ref err) = auth {
                        warn!(?mechanism, ?err, "authentication failed");
                    }

                    if auth.is_ok() {
                        debug!(?mechanism, "authentication succeeded!");
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    if !client.state.login_supported() {
                        return Err(Error::LoginNotSupportedError);
                    }

                    debug!("trying login…");

                    client
                        .login(self.config.login.as_str(), passwd.as_str())
                        .await
                        .map_err(Error::LoginError)?;

                    debug!("login succeeded!");
                }
            }
            #[cfg(feature = "oauth2")]
            ImapAuthConfig::OAuth2(oauth2) => {
                debug!("using OAuth 2.0 authentication");

                match oauth2.method {
                    OAuth2Method::XOAuth2 => {
                        if !client.state.supports_auth_mechanism(AuthMechanism::XOAuth2) {
                            let auth = client.state.supported_auth_mechanisms().cloned().collect();
                            return Err(Error::AuthenticateXOAuth2NotSupportedError(auth));
                        }

                        debug!("using XOAUTH2 auth mechanism");

                        let access_token = match self.credentials.as_ref() {
                            Some(access_token) => access_token.to_string(),
                            None => oauth2
                                .access_token()
                                .await
                                .map_err(Error::RefreshAccessTokenError)?,
                        };

                        let auth = client
                            .authenticate_xoauth2(self.config.login.as_str(), access_token.as_str())
                            .await;

                        if auth.is_err() {
                            warn!("authentication failed, refreshing access token and retrying…");

                            let access_token = oauth2
                                .refresh_access_token()
                                .await
                                .map_err(Error::RefreshAccessTokenError)?;

                            client
                                .authenticate_xoauth2(
                                    self.config.login.as_str(),
                                    access_token.as_str(),
                                )
                                .await
                                .map_err(Error::AuthenticateXOauth2Error)?;

                            self.credentials = Some(access_token);
                        }
                    }
                    OAuth2Method::OAuthBearer => {
                        if !client
                            .state
                            .supports_auth_mechanism("OAUTHBEARER".try_into().unwrap())
                        {
                            let auth = client.state.supported_auth_mechanisms().cloned().collect();
                            return Err(Error::AuthenticateOAuthBearerNotSupportedError(auth));
                        }

                        debug!("using OAUTHBEARER auth mechanism");

                        let access_token = match self.credentials.as_ref() {
                            Some(access_token) => access_token.to_string(),
                            None => oauth2
                                .access_token()
                                .await
                                .map_err(Error::RefreshAccessTokenError)?,
                        };

                        let auth = client
                            .authenticate_oauthbearer(
                                self.config.login.as_str(),
                                self.config.host.as_str(),
                                self.config.port,
                                access_token.as_str(),
                            )
                            .await;

                        if auth.is_err() {
                            warn!("authentication failed, refreshing access token and retrying");

                            let access_token = oauth2
                                .refresh_access_token()
                                .await
                                .map_err(Error::RefreshAccessTokenError)?;

                            client
                                .authenticate_oauthbearer(
                                    self.config.login.as_str(),
                                    self.config.host.as_str(),
                                    self.config.port,
                                    access_token.as_str(),
                                )
                                .await
                                .map_err(Error::AuthenticateOAuthBearerError)?;

                            self.credentials = Some(access_token);
                        }
                    }
                }
            }
        };

        if self.config.send_id_after_auth() {
            let params = ID_PARAMS.clone();
            debug!(?params, "client identity");

            let params = client
                .id(Some(ID_PARAMS.clone()))
                .await
                .map_err(Error::ExchangeIdsError)?;

            debug!(?params, "server identity");
        }

        // TODO: make it customizable
        //
        // debug!("enabling UTF8 capability…");
        //
        // client
        //     .enable(Some(CapabilityEnable::Utf8(Utf8Kind::Accept)))
        //     .await
        //     .map_err(Error::EnableCapabilityError)?;

        Ok(client)
    }
}
