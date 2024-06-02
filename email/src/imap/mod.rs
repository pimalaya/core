pub mod config;
mod error;

use std::{collections::HashMap, env, fmt, num::NonZeroU32, ops::Deref, sync::Arc, time::Duration};

use async_trait::async_trait;
use imap_client::{
    tasks::tasks::select::SelectDataUnvalidated,
    types::{
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
    Client, ClientError,
};
use once_cell::sync::Lazy;
use tokio::sync::{oneshot, Mutex};

use self::config::{ImapAuthConfig, ImapConfig};
#[doc(inline)]
pub use self::error::{Error, Result};
use crate::{
    account::config::{oauth2::OAuth2Method, AccountConfig},
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    debug,
    envelope::{
        get::{imap::GetImapEnvelope, GetEnvelope},
        imap::FETCH_ENVELOPES,
        list::{imap::ListImapEnvelopes, ListEnvelopes},
        thread::{imap::ThreadImapEnvelopes, ThreadEnvelopes},
        watch::{imap::WatchImapEnvelopes, WatchEnvelopes},
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
    warn, AnyResult,
};

macro_rules! retry {
    ($self:ident, $task:expr, $err:expr) => {{
        let mut retried = false;
        loop {
            match $task {
                Err(err) if retried => {
                    break Err($err(err));
                }
                Err(ClientError::Stream(err)) => {
                    println!("err: {:#?}", err);
                    warn!("{err}, re-building IMAP client…");
                    $self.client = $self.client_builder.build().await?;
                    retried = true;
                    continue;
                }
                Err(err) => {
                    break Err($err(err));
                }
                Ok(output) => {
                    break Ok(output);
                }
            }
        }
    }};
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
                "mailto:~soywod/pimalaya@lists.sr.ht".try_into().unwrap(),
            )),
        ),
    ]
});

/// The IMAP backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`ImapContextSync`].
pub struct ImapContext {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The next gen IMAP client builder.
    client_builder: ImapClientBuilder,

    /// The next gen IMAP client.
    client: Client,
}

impl fmt::Debug for ImapContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImapContext")
            .field("imap_config", &self.imap_config)
            .finish_non_exhaustive()
    }
}

impl ImapContext {
    pub async fn create_mailbox(&mut self, mbox: impl ToString) -> Result<()> {
        retry! {
            self,
            self.client.create(mbox.to_string()).await,
            Error::CreateMailboxError
        }
    }

    pub async fn list_all_mailboxes(&mut self, config: &AccountConfig) -> Result<Folders> {
        let mboxes = retry! {
            self,
            self.client.list("", "*").await,
            Error::ListMailboxesError
        }?;

        Ok(Folders::from_imap_mailboxes(config, mboxes))
    }

    pub async fn select_mailbox(&mut self, mbox: impl ToString) -> Result<SelectDataUnvalidated> {
        retry! {
            self,
            self.client.select(mbox.to_string()).await,
            Error::SelectMailboxError
        }
    }

    pub async fn examine_mailbox(&mut self, mbox: impl ToString) -> Result<SelectDataUnvalidated> {
        retry! {
            self,
            self.client.examine(mbox.to_string()).await,
            Error::ExamineMailboxError
        }
    }

    pub async fn expunge_mailbox(&mut self, mbox: impl ToString) -> Result<usize> {
        self.select_mailbox(mbox).await?;

        let expunged = retry! {
            self,
            self.client.expunge().await,
            Error::ExpungeMailboxError
        }?;

        Ok(expunged.len())
    }

    pub async fn purge_mailbox(&mut self, mbox: impl ToString) -> Result<usize> {
        self.select_mailbox(mbox).await?;
        self.add_deleted_flag_silently((..).into()).await?;

        let expunged = retry! {
            self,
            self.client.expunge().await,
            Error::ExpungeMailboxError
        }?;

        Ok(expunged.len())
    }

    pub async fn delete_mailbox(&mut self, mbox: impl ToString) -> Result<()> {
        retry! {
            self,
            self.client.delete(mbox.to_string()).await,
            Error::DeleteMailboxError
        }
    }

    pub async fn fetch_envelopes(&mut self, uids: SequenceSet) -> Result<Envelopes> {
        let fetches = retry! {
            self,
            self.client.uid_fetch(uids.clone(), FETCH_ENVELOPES.clone()).await,
            Error::FetchMessagesError
        }?;

        Ok(Envelopes::from_imap_data_items(fetches))
    }

    pub async fn fetch_envelopes_map(
        &mut self,
        uids: SequenceSet,
    ) -> Result<HashMap<String, Envelope>> {
        let fetches = retry! {
            self,
            self.client.uid_fetch(uids.clone(), FETCH_ENVELOPES.clone()).await,
            Error::FetchMessagesError
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

    pub async fn fetch_first_envelope(&mut self, uid: u32) -> Result<Envelope> {
        let items = retry! {
            self,
            self.client.uid_fetch_first(uid.try_into().unwrap(), FETCH_ENVELOPES.clone()).await,
            Error::FetchMessagesError
        }?;

        Ok(Envelope::from_imap_data_items(items.as_ref()))
    }

    pub async fn fetch_envelopes_by_sequence(&mut self, seq: SequenceSet) -> Result<Envelopes> {
        let fetches = retry! {
            self,
            self.client.fetch(seq.clone(), FETCH_ENVELOPES.clone()).await,
            Error::FetchMessagesError
        }?;

        Ok(Envelopes::from_imap_data_items(fetches))
    }

    pub async fn fetch_all_envelopes(&mut self) -> Result<Envelopes> {
        self.fetch_envelopes_by_sequence((..).into()).await
    }

    pub async fn sort_envelopes(
        &mut self,
        sort_criteria: impl IntoIterator<Item = SortCriterion> + Clone,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Envelopes> {
        let fetches = retry! {
            self,
            self.client.uid_sort_or_fallback(
                sort_criteria.clone(),
                search_criteria.clone(),
                FETCH_ENVELOPES.clone(),
            ).await,
            Error::FetchMessagesError
        }?;

        Ok(Envelopes::from(fetches))
    }

    pub async fn thread_envelopes(
        &mut self,
        search_criteria: impl IntoIterator<Item = SearchKey<'static>> + Clone,
    ) -> Result<Vec<Thread>> {
        retry! {
            self,
            self.client.uid_thread(
                ThreadingAlgorithm::References,
                search_criteria.clone(),
            ).await,
            Error::ThreadMessagesError
        }
    }

    pub async fn idle(
        &mut self,
        wait_for_shutdown_request: &mut oneshot::Receiver<()>,
    ) -> Result<()> {
        let tag = self.client.enqueue_idle();

        tokio::select! {
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

    pub async fn add_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry! {
            self,
            self.client.uid_store(uids.clone(), StoreType::Add, flags.clone()).await,
            Error::StoreFlagsError
        }
    }

    pub async fn add_deleted_flag(
        &mut self,
        uids: SequenceSet,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry! {
            self,
            self.client.uid_store(uids.clone(), StoreType::Add, Some(Flag::Deleted)).await,
            Error::StoreFlagsError
        }
    }

    pub async fn add_deleted_flag_silently(&mut self, uids: SequenceSet) -> Result<()> {
        retry! {
            self,
            self.client.uid_silent_store(uids.clone(), StoreType::Add, Some(Flag::Deleted)).await,
            Error::StoreFlagsError
        }
    }

    pub async fn add_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        retry! {
            self,
            self.client.uid_silent_store(uids.clone(), StoreType::Add, flags.clone()).await,
            Error::StoreFlagsError
        }
    }

    pub async fn set_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry! {
            self,
            self.client.uid_store(uids.clone(), StoreType::Replace, flags.clone()).await,
            Error::StoreFlagsError
        }
    }

    pub async fn set_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        retry! {
            self,
            self.client.uid_silent_store(uids.clone(), StoreType::Replace, flags.clone()).await,
            Error::StoreFlagsError
        }
    }

    pub async fn remove_flags(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<HashMap<NonZeroU32, Vec1<MessageDataItem<'static>>>> {
        retry! {
            self,
            self.client.uid_store(uids.clone(), StoreType::Remove, flags.clone()).await,
            Error::StoreFlagsError
        }
    }

    pub async fn remove_flags_silently(
        &mut self,
        uids: SequenceSet,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
    ) -> Result<()> {
        retry! {
            self,
            self.client.uid_silent_store(uids.clone(), StoreType::Remove, flags.clone()).await,
            Error::StoreFlagsError
        }
    }

    pub async fn add_message(
        &mut self,
        mbox: impl ToString,
        flags: impl IntoIterator<Item = Flag<'static>> + Clone,
        msg: impl AsRef<[u8]> + Clone,
    ) -> Result<NonZeroU32> {
        let id = retry! {
            self,
            self.client.appenduid_or_fallback(mbox.to_string(), flags.clone(), msg.clone()).await,
            Error::StoreFlagsError
        }?;

        id.ok_or(Error::FindAppendedMessageUidError)
    }

    pub async fn fetch_messages(&mut self, uids: SequenceSet) -> Result<Messages> {
        let mut fetches = retry! {
            self,
            self.client.uid_fetch(uids.clone(), FETCH_MESSAGES.clone()).await,
            Error::StoreFlagsError
        }?;

        let fetches: Vec<_> = uids
            .iter(NonZeroU32::MAX)
            .filter_map(|ref uid| fetches.remove(uid))
            .collect();

        Ok(Messages::from(fetches))
    }

    pub async fn peek_messages(&mut self, uids: SequenceSet) -> Result<Messages> {
        let mut fetches = retry! {
            self,
            self.client.uid_fetch(uids.clone(), PEEK_MESSAGES.clone()).await,
            Error::StoreFlagsError
        }?;

        let fetches: Vec<_> = uids
            .iter(NonZeroU32::MAX)
            .filter_map(|ref uid| fetches.remove(uid))
            .collect();

        Ok(Messages::from(fetches))
    }

    pub async fn copy_messages(&mut self, uids: SequenceSet, mbox: impl ToString) -> Result<()> {
        retry! {
            self,
            self.client.uid_copy(uids.clone(), mbox.to_string()).await,
            Error::CopyMessagesError
        }
    }

    pub async fn move_messages(&mut self, uids: SequenceSet, mbox: impl ToString) -> Result<()> {
        retry! {
            self,
            self.client.uid_move_or_fallback(uids.clone(), mbox.to_string()).await,
            Error::MoveMessagesError
        }
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

    /// The current IMAP session.
    inner: Arc<Mutex<ImapContext>>,
}

impl Deref for ImapContextSync {
    type Target = Arc<Mutex<ImapContext>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
}

impl ImapContextBuilder {
    pub fn new(account_config: Arc<AccountConfig>, imap_config: Arc<ImapConfig>) -> Self {
        Self {
            account_config,
            imap_config,
            prebuilt_credentials: None,
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
}

#[cfg(feature = "account-sync")]
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

    fn thread_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ThreadEnvelopes>> {
        Some(Arc::new(ThreadImapEnvelopes::some_new_boxed))
    }

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

    // #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    async fn build(self) -> AnyResult<Self::Context> {
        let mut client_builder =
            ImapClientBuilder::new(self.imap_config.clone(), self.prebuilt_credentials);

        let client = client_builder.build().await?;

        let ctx = ImapContext {
            account_config: self.account_config.clone(),
            imap_config: self.imap_config.clone(),
            client_builder,
            client,
        };

        Ok(ImapContextSync {
            account_config: self.account_config,
            imap_config: self.imap_config,
            inner: Arc::new(Mutex::new(ctx)),
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
        let mut ctx = self.ctx.lock().await;
        ctx.client.noop().await.map_err(Error::ExecuteNoOpError)?;
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
    pub async fn build(&mut self) -> Result<Client> {
        let mut client = match &self.config.encryption {
            Some(ImapEncryptionKind::None) | None => {
                Client::insecure(&self.config.host, self.config.port)
                    .await
                    .unwrap()
            }
            Some(ImapEncryptionKind::StartTls) => {
                Client::starttls(&self.config.host, self.config.port)
                    .await
                    .unwrap()
            }
            Some(ImapEncryptionKind::Tls) => Client::tls(&self.config.host, self.config.port)
                .await
                .unwrap(),
        };

        client.set_some_idle_timeout(self.config.find_watch_timeout().map(Duration::from_secs));

        match &self.config.auth {
            ImapAuthConfig::Passwd(passwd) => {
                if !client.supports_auth_mechanism(AuthMechanism::Plain) {
                    let auth = client.supported_auth_mechanisms().cloned().collect();
                    return Err(Error::AuthenticatePlainNotSupportedError(auth));
                }

                debug!("using PLAIN auth mechanism");

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

                client
                    .authenticate_plain(self.config.login.as_str(), passwd.as_str())
                    .await
                    .map_err(Error::AuthenticatePlainError)?;
            }
            ImapAuthConfig::OAuth2(oauth2) => match oauth2.method {
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
                        warn!("authentication failed, refreshing access token and retrying");

                        let access_token = oauth2
                            .refresh_access_token()
                            .await
                            .map_err(Error::RefreshAccessTokenError)?;

                        client
                            .authenticate_xoauth2(self.config.login.as_str(), access_token.as_str())
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
            },
        };

        let server_id = client
            .id(Some(ID_PARAMS.clone()))
            .await
            .map_err(Error::ExchangeIdsError)?;

        debug!(?server_id, "server identity");

        Ok(client)
    }
}
