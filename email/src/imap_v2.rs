use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use imap_client::Client;
use imap_next::imap_types::auth::AuthMechanism;
use tokio::{
    sync::{Mutex, MutexGuard},
    time::sleep,
};

#[cfg(feature = "oauth2")]
use crate::account::config::oauth2::OAuth2Method;
#[cfg(feature = "oauth2")]
use crate::warn;
use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::BackendFeature,
    },
    debug,
    envelope::list::{imap::ListImapEnvelopesV2, ListEnvelopes},
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        Error, Result,
    },
    AnyResult,
};

/// The sync version of the IMAP backend context.
///
/// This is just an IMAP session wrapped into a mutex, so the same
/// IMAP session can be shared and updated across multiple threads.
#[derive(Debug, Clone)]
pub struct ImapContextSyncV2 {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The current IMAP session.
    inner: Vec<Arc<Mutex<ImapContextV2>>>,
}

impl ImapContextSyncV2 {
    pub async fn client(&self) -> MutexGuard<'_, ImapContextV2> {
        debug!("try get client");

        loop {
            if let Some(ctx) = self.inner.iter().find_map(|ctx| ctx.try_lock().ok()) {
                break ctx;
            };

            debug!("no client available, sleep for 1s");
            sleep(Duration::from_secs(1)).await;
        }
    }
}

impl BackendContext for ImapContextSyncV2 {}

pub struct ImapContextV2 {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The next gen IMAP client builder.
    pub client_builder: ImapClientBuilder,

    client: Client,
}

impl Deref for ImapContextV2 {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for ImapContextV2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl fmt::Debug for ImapContextV2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImapContextV2")
            .field("imap_config", &self.imap_config)
            .finish_non_exhaustive()
    }
}

/// The IMAP backend context builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ImapContextBuilderV2 {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The IMAP configuration.
    pub imap_config: Arc<ImapConfig>,

    /// The prebuilt IMAP credentials.
    prebuilt_credentials: Option<String>,

    pool_size: u8,
}

impl ImapContextBuilderV2 {
    pub fn new(account_config: Arc<AccountConfig>, imap_config: Arc<ImapConfig>) -> Self {
        Self {
            account_config,
            imap_config,
            prebuilt_credentials: None,
            pool_size: 1,
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

    pub async fn with_pool_size(mut self, pool_size: u8) -> Self {
        self.pool_size = pool_size;
        self
    }
}

#[cfg(feature = "sync")]
impl crate::sync::hash::SyncHash for ImapContextBuilderV2 {
    fn sync_hash(&self, state: &mut std::hash::DefaultHasher) {
        self.imap_config.sync_hash(state);
    }
}

#[async_trait]
impl BackendContextBuilder for ImapContextBuilderV2 {
    type Context = ImapContextSyncV2;

    fn list_envelopes(&self) -> Option<BackendFeature<Self::Context, dyn ListEnvelopes>> {
        Some(Arc::new(ListImapEnvelopesV2::some_new_boxed))
    }

    async fn build(self) -> AnyResult<Self::Context> {
        let client_builder =
            ImapClientBuilder::new(self.imap_config.clone(), self.prebuilt_credentials);

        let inner = FuturesUnordered::from_iter((0..self.pool_size).map(move |_| {
            let mut client_builder = client_builder.clone();
            tokio::spawn(async move {
                let client = client_builder.build().await.unwrap();
                (client_builder, client)
            })
        }))
        .filter_map(|res| async { res.ok() })
        .map(|(client_builder, client)| ImapContextV2 {
            account_config: self.account_config.clone(),
            imap_config: self.imap_config.clone(),
            client_builder,
            client,
        })
        .map(Mutex::new)
        .map(Arc::new)
        .collect()
        .await;

        Ok(ImapContextSyncV2 {
            account_config: self.account_config,
            imap_config: self.imap_config,
            inner,
        })
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

        Ok(client)
    }
}
