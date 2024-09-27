pub mod config;
mod error;

use std::{
    collections::HashMap,
    env, fmt,
    num::NonZeroU32,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use imap_client::{
    tasks::{tasks::select::SelectDataUnvalidated, SchedulerError},
    Client, ClientError,
};
use imap_next::{
    imap_types::{
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
};
use once_cell::sync::Lazy;
use paste::paste;
use tokio::{
    select,
    sync::{oneshot, Mutex, MutexGuard},
    time::sleep,
};

use self::config::{ImapAuthConfig, ImapConfig};
#[doc(inline)]
pub use self::error::{Error, Result};
#[cfg(feature = "oauth2")]
use crate::account::config::oauth2::OAuth2Method;
#[cfg(feature = "thread")]
use crate::envelope::thread::{imap::ThreadImapEnvelopes, ThreadEnvelopes};
#[cfg(feature = "watch")]
use crate::envelope::watch::{imap::WatchImapEnvelopes, WatchEnvelopes};
#[cfg(feature = "oauth2")]
use crate::warn;
use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    debug,
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
    imap::config::ImapEncryptionKind,
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
    retry::{Retry, RetryState},
    AnyResult,
};

macro_rules! retry {
    ($self:ident, $task:expr, $err:ident) => {
        paste! {{
            let mut retry = Retry::default();

            loop {
                match retry.next(retry.timeout($task).await) {
                    RetryState::Retry => {
                        debug!(attempt = retry.attempts, "request timed out");
                        continue;
                    }
                    RetryState::TimedOut => {
                        break Err(Error::[<$err TimedOutError>]);
                    }
                    RetryState::Ok(Ok(res)) => {
                        break Ok(res);
                    }
                    RetryState::Ok(Err(ClientError::Stream(StreamError::State(SchedulerError::UnexpectedByeResponse(bye))))) => {
                        #[cfg(feature = "tracing")]
                        tracing::debug!(reason = bye.text.to_string(), "connection closed");

                        #[cfg(feature = "tracing")]
			tracing::debug!("re-connecting…");

			$self.client = $self.client_builder.build().await?;

			if let Some(mbox) = &$self.mailbox {
			    $self.client.select(mbox.clone()).await.map_err(Error::SelectMailboxError)?;
			}

			retry.attempts = 0;
			continue;
                    }
                    RetryState::Ok(Err(err)) => {
			break Err(Error::[<$err Error>](err));
                    }
		}
            }
        }}
    };
}

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

/// The IMAP backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`ImapContextSync`].
pub struct ImapContext {
    pub id: u8,

    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The next gen IMAP client builder.
    pub client_builder: ImapClientBuilder,

    /// The next gen IMAP client.
    client: Client,

    /// The selected mailbox.
    mailbox: Option<String>,
}

impl fmt::Debug for ImapContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImapContext")
            .field("imap_config", &self.imap_config)
            .finish_non_exhaustive()
    }
}

impl ImapContext {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn select_mailbox(&mut self, mbox: impl ToString) -> Result<SelectDataUnvalidated> {
        let data = retry!(self, self.client.select(mbox.to_string()), SelectMailbox)?;
        self.mailbox = Some(mbox.to_string());
        Ok(data)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn examine_mailbox(&mut self, mbox: impl ToString) -> Result<SelectDataUnvalidated> {
        retry!(self, self.client.examine(mbox.to_string()), ExamineMailbox)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn create_mailbox(&mut self, mbox: impl ToString) -> Result<()> {
        retry!(self, self.client.create(mbox.to_string()), CreateMailbox)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn list_all_mailboxes(&mut self, config: &AccountConfig) -> Result<Folders> {
        let mboxes = retry!(self, self.client.list("", "*"), ListMailboxes)?;
        let folders = Folders::from_imap_mailboxes(config, mboxes);
        Ok(folders)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn expunge_mailbox(&mut self, mbox: impl ToString) -> Result<usize> {
        self.select_mailbox(mbox).await?;
        let expunged = retry!(self, self.client.expunge(), ExpungeMailbox)?;
        Ok(expunged.len())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn purge_mailbox(&mut self, mbox: impl ToString) -> Result<usize> {
        self.select_mailbox(mbox).await?;
        self.add_deleted_flag_silently("1:*".try_into().unwrap())
            .await?;
        let expunged = retry!(self, self.client.expunge(), ExpungeMailbox)?;
        Ok(expunged.len())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn delete_mailbox(&mut self, mbox: impl ToString) -> Result<()> {
        retry!(self, self.client.delete(mbox.to_string()), DeleteMailbox)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn fetch_envelopes(&mut self, uids: SequenceSet) -> Result<Envelopes> {
        let fetches = retry!(
            self,
            self.client.uid_fetch(uids.clone(), FETCH_ENVELOPES.clone()),
            FetchMessages
        )?;

        Ok(Envelopes::from_imap_data_items(fetches))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn fetch_envelopes_map(
        &mut self,
        uids: SequenceSet,
    ) -> Result<HashMap<String, Envelope>> {
        let fetches = retry!(
            self,
            self.client.uid_fetch(uids.clone(), FETCH_ENVELOPES.clone()),
            FetchMessages
        )?;

        let map = fetches
            .into_values()
            .map(|items| {
                let envelope = Envelope::from_imap_data_items(items.as_ref());
                (envelope.id.clone(), envelope)
            })
            .collect();

        Ok(map)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn fetch_first_envelope(&mut self, uid: u32) -> Result<Envelope> {
        let items = retry!(
            self,
            self.client
                .uid_fetch_first(uid.try_into().unwrap(), FETCH_ENVELOPES.clone()),
            FetchMessages
        )?;

        Ok(Envelope::from_imap_data_items(items.as_ref()))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn fetch_envelopes_by_sequence(&mut self, seq: SequenceSet) -> Result<Envelopes> {
        let fetches = retry!(
            self,
            self.client.fetch(seq.clone(), FETCH_ENVELOPES.clone()),
            FetchMessages
        )?;

        Ok(Envelopes::from_imap_data_items(fetches))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn fetch_all_envelopes(&mut self) -> Result<Envelopes> {
        self.fetch_envelopes_by_sequence("1:*".try_into().unwrap())
            .await
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn sort_envelopes(
        &mut self,
        sort_criteria: impl IntoIterator<Item = SortCriterion> + Clone,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Envelopes> {
        let fetches = retry!(
            self,
            self.client.uid_sort_or_fallback(
                sort_criteria.clone(),
                search_criteria.clone(),
                FETCH_ENVELOPES.clone(),
            ),
            FetchMessages
        )?;

        Ok(Envelopes::from(fetches))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn thread_envelopes(
        &mut self,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Vec<Thread>> {
        retry!(
            self,
            self.client
                .uid_thread(ThreadingAlgorithm::References, search_criteria.clone(),),
            ThreadMessages
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn idle(
        &mut self,
        wait_for_shutdown_request: &mut oneshot::Receiver<()>,
    ) -> Result<()> {
        let tag = self.client.enqueue_idle();

        select! {
            output = self.client.idle(tag.clone()) => {
                output.map_err(Error::StartIdleError)?;
                Ok(())
            },
            _ = wait_for_shutdown_request => {
                debug!("shutdown requested, sending done command…");
                self.client.idle_done(tag.clone()).await.map_err(Error::StopIdleError)?;
                Err(Error::IdleInterruptedError)
            }
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn add_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry!(
            self,
            self.client
                .uid_store(uids.clone(), StoreType::Add, flags.clone()),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn add_deleted_flag(
        &mut self,
        uids: SequenceSet,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry!(
            self,
            self.client
                .uid_store(uids.clone(), StoreType::Add, Some(Flag::Deleted)),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn add_deleted_flag_silently(&mut self, uids: SequenceSet) -> Result<()> {
        retry!(
            self,
            self.client
                .uid_silent_store(uids.clone(), StoreType::Add, Some(Flag::Deleted)),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn add_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        retry!(
            self,
            self.client
                .uid_silent_store(uids.clone(), StoreType::Add, flags.clone()),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn set_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry!(
            self,
            self.client
                .uid_store(uids.clone(), StoreType::Replace, flags.clone()),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn set_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        retry!(
            self,
            self.client
                .uid_silent_store(uids.clone(), StoreType::Replace, flags.clone()),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn remove_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry!(
            self,
            self.client
                .uid_store(uids.clone(), StoreType::Remove, flags.clone()),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn remove_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        retry!(
            self,
            self.client
                .uid_silent_store(uids.clone(), StoreType::Remove, flags.clone()),
            StoreFlags
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn add_message(
        &mut self,
        mbox: impl ToString,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
        msg: impl AsRef<[u8]> + Clone,
    ) -> Result<NonZeroU32> {
        let id = retry!(
            self,
            self.client
                .appenduid_or_fallback(mbox.to_string(), flags.clone(), msg.clone()),
            StoreFlags
        )?;

        id.ok_or(Error::FindAppendedMessageUidError)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn fetch_messages(&mut self, uids: SequenceSet) -> Result<Messages> {
        let mut fetches = retry!(
            self,
            self.client.uid_fetch(uids.clone(), FETCH_MESSAGES.clone()),
            FetchMessages
        )?;

        let fetches: Vec<_> = uids
            .iter(NonZeroU32::MAX)
            .filter_map(|ref uid| fetches.remove(uid))
            .collect();

        Ok(Messages::from(fetches))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn peek_messages(&mut self, uids: SequenceSet) -> Result<Messages> {
        let mut fetches = retry!(
            self,
            self.client.uid_fetch(uids.clone(), PEEK_MESSAGES.clone()),
            FetchMessages
        )?;

        let fetches: Vec<_> = uids
            .iter(NonZeroU32::MAX)
            .filter_map(|ref uid| fetches.remove(uid))
            .collect();

        Ok(Messages::from(fetches))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn copy_messages(&mut self, uids: SequenceSet, mbox: impl ToString) -> Result<()> {
        retry!(
            self,
            self.client.uid_copy(uids.clone(), mbox.to_string()),
            CopyMessages
        )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(client = self.id)))]
    pub async fn move_messages(&mut self, uids: SequenceSet, mbox: impl ToString) -> Result<()> {
        retry!(
            self,
            self.client
                .uid_move_or_fallback(uids.clone(), mbox.to_string()),
            MoveMessages
        )
    }
}

impl Deref for ImapContext {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for ImapContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

/// The sync version of the IMAP backend context.
///
/// This is just an IMAP session wrapped into a mutex, so the same
/// IMAP session can be shared and updated across multiple threads.
#[derive(Debug, Clone)]
pub struct ImapContextSync {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    contexts: Vec<Arc<Mutex<ImapContext>>>,
}

impl ImapContextSync {
    #[cfg(not(feature = "tracing"))]
    pub async fn client(&self) -> MutexGuard<'_, ImapContext> {
        loop {
            match self.contexts.iter().find_map(|ctx| ctx.try_lock().ok()) {
                Some(ctx) => break ctx,
                None => sleep(Duration::from_secs(1)).await,
            }
        }
    }

    pub async fn client(&self) -> MutexGuard<'_, ImapContext> {
        loop {
            let lock = self.contexts.iter().find_map(|ctx| ctx.try_lock().ok());

            if let Some(ctx) = lock {
                #[cfg(feature = "tracing")]
                {
                    let total = self.contexts.len();
                    let id = ctx.id;
                    tracing::debug!("client {id}/{total} is free, locking it");
                }

                break ctx;
            } else {
                #[cfg(feature = "tracing")]
                tracing::trace!("no free client, sleeping for 1s");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

impl BackendContext for ImapContextSync {}

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
    type Context = ImapContextSync;

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

        #[cfg(feature = "tracing")]
        tracing::debug!("building {} IMAP clients", self.pool_size);

        let contexts = FuturesUnordered::from_iter((0..self.pool_size).map(move |i| {
            let mut client_builder = client_builder.clone();
            tokio::spawn(async move {
                let client = client_builder.build().await?;
                Ok((i + 1, client_builder, client))
            })
        }))
        .map(|res| match res {
            Err(err) => Err(Error::JoinClientError(err)),
            Ok(Err(err)) => Err(Error::BuildClientError(Box::new(err))),
            Ok(Ok((id, client_builder, client))) => Ok(Arc::new(Mutex::new(ImapContext {
                id,
                account_config: self.account_config.clone(),
                imap_config: self.imap_config.clone(),
                client_builder,
                client,
                mailbox: None,
            }))),
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<_>>()?;

        Ok(ImapContextSync {
            account_config: self.account_config,
            imap_config: self.imap_config,
            contexts,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CheckUpImap {
    ctx: ImapContextSync,
}

impl CheckUpImap {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn CheckUp> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn CheckUp>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CheckUp for CheckUpImap {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn check_up(&self) -> AnyResult<()> {
        debug!("executing check up backend feature");
        let mut client = self.ctx.client().await;
        client.noop().await.map_err(Error::ExecuteNoOpError)?;
        Ok(())
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
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(name = "client::build", skip(self))
    )]
    pub async fn build(&mut self) -> Result<Client> {
        let mut client = match &self.config.encryption {
            Some(ImapEncryptionKind::None) | None => {
                Client::insecure(&self.config.host, self.config.port)
                    .await
                    .map_err(|err| {
                        let host = self.config.host.clone();
                        let port = self.config.port.clone();
                        Error::BuildInsecureClientError(err, host, port)
                    })?
            }
            Some(ImapEncryptionKind::StartTls) => {
                Client::starttls(&self.config.host, self.config.port)
                    .await
                    .map_err(|err| {
                        let host = self.config.host.clone();
                        let port = self.config.port.clone();
                        Error::BuildStartTlsClientError(err, host, port)
                    })?
            }
            Some(ImapEncryptionKind::Tls) => Client::tls(&self.config.host, self.config.port)
                .await
                .map_err(|err| {
                    let host = self.config.host.clone();
                    let port = self.config.port.clone();
                    Error::BuildTlsClientError(err, host, port)
                })?,
        };

        client.set_some_idle_timeout(self.config.find_watch_timeout().map(Duration::from_secs));

        match &self.config.auth {
            ImapAuthConfig::Passwd(passwd) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("using password authentication");

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

                let mechanisms: Vec<_> = client.supported_auth_mechanisms().cloned().collect();
                let mut authenticated = false;

                #[cfg(feature = "tracing")]
                tracing::debug!(?mechanisms, "supported auth mechanisms");

                for mechanism in mechanisms {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(?mechanism, "trying auth mechanism…");

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

                    #[cfg(feature = "tracing")]
                    if let Err(ref err) = auth {
                        tracing::warn!(?mechanism, ?err, "authentication failed");
                    }

                    if auth.is_ok() {
                        #[cfg(feature = "tracing")]
                        tracing::debug!(?mechanism, "authentication succeeded!");
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    if !client.login_supported() {
                        return Err(Error::LoginNotSupportedError);
                    }

                    #[cfg(feature = "tracing")]
                    tracing::debug!("trying login…");

                    client
                        .login(self.config.login.as_str(), passwd.as_str())
                        .await
                        .map_err(Error::LoginError)?;

                    #[cfg(feature = "tracing")]
                    tracing::debug!("login succeeded!");
                }
            }
            #[cfg(feature = "oauth2")]
            ImapAuthConfig::OAuth2(oauth2) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("using OAuth 2.0 authentication");

                match oauth2.method {
                    OAuth2Method::XOAuth2 => {
                        if !client.supports_auth_mechanism(AuthMechanism::XOAuth2) {
                            let auth = client.supported_auth_mechanisms().cloned().collect();
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
                        if !client.supports_auth_mechanism("OAUTHBEARER".try_into().unwrap()) {
                            let auth = client.supported_auth_mechanisms().cloned().collect();
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
            #[cfg(feature = "tracing")]
            {
                let params = ID_PARAMS.clone();
                tracing::debug!(?params, "client identity");
            }

            #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
            let params = client
                .id(Some(ID_PARAMS.clone()))
                .await
                .map_err(Error::ExchangeIdsError)?;

            debug!(?params, "server identity");
        }

        // TODO: make it customizable
        //
        // #[cfg(feature = "tracing")]
        // tracing::debug!("enabling UTF8 capability…");
        //
        // client
        //     .enable(Some(CapabilityEnable::Utf8(Utf8Kind::Accept)))
        //     .await
        //     .map_err(Error::EnableCapabilityError)?;

        Ok(client)
    }
}
