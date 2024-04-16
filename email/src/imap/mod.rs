pub mod config;
mod error;

use crate::{debug, info, warn};
use async_trait::async_trait;
use imap::{Authenticator, Client, ImapConnection, Session, TlsKind};
use std::{ops::Deref, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    account::config::{oauth2::OAuth2Method, AccountConfig},
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    envelope::{
        get::{imap::GetImapEnvelope, GetEnvelope},
        list::{imap::ListImapEnvelopes, ListEnvelopes},
        watch::{imap::WatchImapEnvelopes, WatchEnvelopes},
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
    },
    message::{
        add::{imap::AddImapMessage, AddMessage},
        copy::{imap::CopyImapMessages, CopyMessages},
        delete::{imap::DeleteImapMessages, DeleteMessages},
        get::{imap::GetImapMessages, GetMessages},
        peek::{imap::PeekImapMessages, PeekMessages},
        r#move::{imap::MoveImapMessages, MoveMessages},
        remove::{imap::RemoveImapMessages, RemoveMessages},
    },
    AnyError, AnyResult,
};

use self::config::{ImapAuthConfig, ImapConfig};
#[doc(inline)]
pub use self::error::{Error, Result};

/// The IMAP backend context.
///
/// This context is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`ImapContextSync`].
#[derive(Debug)]
pub struct ImapContext {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The current IMAP session.
    session: Session<Box<dyn ImapConnection>>,
}

impl ImapContext {
    /// Execute the given action on the current IMAP session.
    ///
    /// If an OAuth 2.0 authentication error occurs, the access token
    /// is refreshed and the action is executed once again.
    pub async fn exec<T, E: AnyError>(
        &mut self,
        action: impl Fn(&mut Session<Box<dyn ImapConnection>>) -> imap::Result<T>,
        map_err: impl Fn(imap::Error) -> E,
    ) -> Result<T> {
        let mut retry = ImapRetryState::default();

        loop {
            match action(&mut self.session) {
                Ok(res) => {
                    break Ok(res);
                }
                Err(err) if retry.is_empty() => {
                    warn!("cannot exec IMAP action after 3 attempts, aborting");
                    let err = Box::new(map_err(err));
                    break Err(Error::ExecuteActionRetryError(err));
                }
                Err(err) => {
                    if is_authentication_failed(&err) {
                        match &self.imap_config.auth {
                            ImapAuthConfig::Passwd(_) => {
                                let err = Box::new(map_err(err));
                                break Err(Error::ExecuteActionPasswordError(err));
                            }
                            ImapAuthConfig::OAuth2(_) if retry.oauth2_access_token_refreshed => {
                                let err = Box::new(map_err(err));
                                break Err(Error::ExecuteActionOAuthError(err));
                            }
                            ImapAuthConfig::OAuth2(oauth2_config) => {
                                oauth2_config
                                    .refresh_access_token()
                                    .await
                                    .map_err(Error::RefreshAccessTokenError)?;
                                self.session = build_session(&self.imap_config, None).await?;
                                retry.set_oauth2_access_token_refreshed();
                                continue;
                            }
                        }
                    }
                    // TODO: find a way to better identify timeout
                    // errors.
                    else {
                        let count = 3 - retry.decrement();
                        warn!("cannot exec imap action: {err}, attempt ({count})");
                        continue;
                    }
                }
            }
        }
    }

    pub async fn noop(&mut self) -> Result<()> {
        self.session.noop().map_err(Error::ExecuteNoOperationError)
    }
}

impl Drop for ImapContext {
    fn drop(&mut self) {
        let _ = self.session.close();
        let _ = self.session.logout();
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
        info!("building new imap context");

        let mut retry = ImapRetryState::default();
        let mut creds = self.prebuilt_credentials;

        let session = loop {
            match build_session(&self.imap_config, creds.as_ref()).await {
                Ok(sess) => {
                    break Ok(sess);
                }
                Err(err) if retry.is_empty() => {
                    warn!("cannot build imap session after 3 attempts, aborting");
                    break Err(err);
                }
                Err(err) => {
                    match &err {
                        Error::AuthenticateImapError(e) if is_authentication_failed(&e) => {
                            match &self.imap_config.auth {
                                ImapAuthConfig::Passwd(_) => {
                                    break Err(err);
                                }
                                ImapAuthConfig::OAuth2(_)
                                    if retry.oauth2_access_token_refreshed =>
                                {
                                    break Err(err);
                                }
                                ImapAuthConfig::OAuth2(config) => {
                                    creds = Some(
                                        config
                                            .refresh_access_token()
                                            .await
                                            .map_err(Error::RefreshAccessTokenError)?,
                                    );
                                    continue;
                                }
                            }
                        }
                        // TODO: find a way to better identify timeout
                        // errors.
                        _ => {
                            let count = 3 - retry.decrement();
                            warn!("cannot build imap session: {err}, attempt ({count})");
                            continue;
                        }
                    }
                }
            }
        }?;

        let ctx = ImapContext {
            account_config: self.account_config.clone(),
            imap_config: self.imap_config.clone(),
            session,
        };

        Ok(ImapContextSync {
            account_config: self.account_config,
            imap_config: self.imap_config,
            inner: Arc::new(Mutex::new(ctx)),
        })
    }
}

/// XOAUTH2 IMAP authenticator.
///
/// This struct is needed to implement the [imap::Authenticator]
/// trait.
struct XOAuth2 {
    user: String,
    access_token: String,
}

impl XOAuth2 {
    pub fn new(user: String, access_token: String) -> Self {
        Self { user, access_token }
    }
}

impl Authenticator for XOAuth2 {
    type Response = String;

    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

/// OAUTHBEARER IMAP authenticator.
///
/// This struct is needed to implement the [imap::Authenticator]
/// trait.
struct OAuthBearer {
    user: String,
    host: String,
    port: u16,
    access_token: String,
}

impl OAuthBearer {
    pub fn new(user: String, host: String, port: u16, access_token: String) -> Self {
        Self {
            user,
            host,
            port,
            access_token,
        }
    }
}

impl Authenticator for OAuthBearer {
    type Response = String;

    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "n,a={},\x01host={}\x01port={}\x01auth=Bearer {}\x01\x01",
            self.user, self.host, self.port, self.access_token
        )
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
    async fn check_up(&self) -> AnyResult<()> {
        let mut ctx = self.ctx.lock().await;
        ctx.noop().await?;
        Ok(())
    }
}

struct ImapRetryState {
    pub count: usize,
    pub oauth2_access_token_refreshed: bool,
}

impl ImapRetryState {
    fn decrement(&mut self) -> usize {
        if self.count > 0 {
            self.count -= 1
        }

        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn set_oauth2_access_token_refreshed(&mut self) {
        self.count = 3;
        self.oauth2_access_token_refreshed = true;
    }
}

impl Default for ImapRetryState {
    fn default() -> Self {
        Self {
            count: 3,
            oauth2_access_token_refreshed: false,
        }
    }
}

/// Creates a new session from an IMAP configuration and optional
/// pre-built credentials.
///
/// Pre-built credentials are useful to prevent building them
/// every time a new session is created. The main use case is for
/// the synchronization, where multiple sessions can be created in
/// a row.
pub async fn build_session(
    imap_config: &ImapConfig,
    credentials: Option<&String>,
) -> Result<Session<Box<dyn ImapConnection>>> {
    let mut session = match &imap_config.auth {
        ImapAuthConfig::Passwd(passwd) => {
            debug!("creating session using login and password");
            let passwd = match credentials {
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
            build_client(imap_config)?
                .login(&imap_config.login, passwd)
                .map_err(|res| Error::LoginImapError(res.0))
        }
        ImapAuthConfig::OAuth2(oauth2_config) => {
            let access_token = match credentials {
                Some(access_token) => access_token.to_string(),
                None => oauth2_config
                    .access_token()
                    .await
                    .map_err(Error::RefreshAccessTokenError)?,
            };
            match oauth2_config.method {
                OAuth2Method::XOAuth2 => {
                    debug!("creating session using xoauth2");
                    let xoauth2 = XOAuth2::new(imap_config.login.clone(), access_token);
                    build_client(imap_config)?
                        .authenticate("XOAUTH2", &xoauth2)
                        .map_err(|(err, _client)| Error::AuthenticateImapError(err))
                }
                OAuth2Method::OAuthBearer => {
                    debug!("creating session using oauthbearer");
                    let bearer = OAuthBearer::new(
                        imap_config.login.clone(),
                        imap_config.host.clone(),
                        imap_config.port,
                        access_token,
                    );
                    build_client(imap_config)?
                        .authenticate("OAUTHBEARER", &bearer)
                        .map_err(|(err, _client)| Error::AuthenticateImapError(err))
                }
            }
        }
    }?;

    Ok(session)
}

/// Creates a client from an IMAP configuration.
fn build_client(imap_config: &ImapConfig) -> Result<Client<Box<dyn ImapConnection>>> {
    let mut client_builder = imap::ClientBuilder::new(&imap_config.host, imap_config.port)
        .tls_kind(TlsKind::Rust)
        .mode(imap_config.encryption.clone().unwrap_or_default().into());

    if imap_config.is_encryption_disabled() {
        client_builder = client_builder.danger_skip_tls_verify(true);
    }

    let client = client_builder.connect().map_err(Error::ConnectImapError)?;

    Ok(client)
}

fn is_authentication_failed(err: &imap::Error) -> bool {
    match err {
        imap::Error::No(no) if no.information.contains("AUTHENTICATIONFAILED") => true,
        // other cases could potentially match authentication failed?
        _ => false,
    }
}
