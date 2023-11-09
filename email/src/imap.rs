use async_trait::async_trait;
use imap::{extensions::idle::SetReadTimeout, Authenticator, Client, Session};
use log::{debug, info, log_enabled, warn, Level};
use once_cell::sync::Lazy;
use rustls::{
    client::{ServerCertVerified, ServerCertVerifier},
    Certificate, ClientConfig, ClientConnection, RootCertStore, StreamOwned,
};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    ops::Deref,
    result,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    account::{AccountConfig, OAuth2Method},
    backend::{BackendConfig, BackendContextBuilder, ImapAuthConfig, ImapConfig},
    Result,
};

/// Errors related to the IMAP backend.
#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot authenticate to imap server")]
    AuthenticateError(#[source] imap::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdError(#[source] secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,
    #[error("cannot login to imap server")]
    LoginError(#[source] imap::Error),
    #[error("cannot connect to imap server")]
    ConnectError(#[source] imap::Error),
    #[error("cannot execute imap action")]
    ExecuteSessionActionError(#[source] Box<dyn std::error::Error + Send>),
}

/// Native certificates store, mostly used by
/// `Backend::tls_handshake()`.
const ROOT_CERT_STORE: Lazy<RootCertStore> = Lazy::new(|| {
    let mut store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().unwrap() {
        store.add(&Certificate(cert.0)).unwrap();
    }
    store
});

/// Alias for the TLS/SSL stream, which is basically a
/// [std::net::TcpStream] wrapped by a [rustls::StreamOwned].
pub type TlsStream = StreamOwned<ClientConnection, TcpStream>;

/// Wrapper around TLS/SSL and TCP streams.
///
/// Since [imap::Session] needs a generic stream type, this wrapper is needed to create the session alias [ImapSession].
#[derive(Debug)]
pub enum ImapSessionStream {
    Tls(TlsStream),
    Tcp(TcpStream),
}

impl SetReadTimeout for ImapSessionStream {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> imap::Result<()> {
        match self {
            Self::Tls(stream) => Ok(stream.get_mut().set_read_timeout(timeout)?),
            Self::Tcp(stream) => stream.set_read_timeout(timeout),
        }
    }
}

impl Read for ImapSessionStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.read(buf),
            Self::Tcp(stream) => stream.read(buf),
        }
    }
}

impl Write for ImapSessionStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.write(buf),
            Self::Tcp(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tls(stream) => stream.flush(),
            Self::Tcp(stream) => stream.flush(),
        }
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

/// The IMAP session builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ImapSessionBuilder {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The IMAP configuration.
    pub imap_config: ImapConfig,

    /// The default credentials.
    default_credentials: Option<String>,

    /// The disable cache flag.
    disable_cache: bool,
}

impl ImapSessionBuilder {
    pub fn new(account_config: AccountConfig, imap_config: ImapConfig) -> Self {
        Self {
            account_config,
            imap_config,
            ..Default::default()
        }
    }

    /// Disable cache flag setter.
    pub fn disable_cache(&mut self, disable_cache: bool) {
        self.disable_cache = disable_cache;
    }

    /// Disable cache flag setter following the builder pattern.
    pub fn with_cache_disabled(mut self, disable_cache: bool) -> Self {
        self.disable_cache = disable_cache;
        self
    }

    /// Credentials setter following the builder pattern.
    pub fn with_credentials(mut self, credentials: Option<String>) -> Self {
        self.default_credentials = credentials;
        self
    }

    /// Default credentials setter following the builder pattern.
    pub async fn with_default_credentials(mut self) -> Result<Self> {
        self.default_credentials = match &self.account_config.backend {
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Some(imap_config.build_credentials().await?)
            }
            _ => None,
        };
        Ok(self)
    }
}

#[async_trait]
impl BackendContextBuilder for ImapSessionBuilder {
    type Context = ImapSessionSync;

    /// Build an IMAP sync session.
    ///
    /// The IMAP session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self) -> Result<Self::Context> {
        info!("building new imap session");
        let creds = self.default_credentials.as_ref();
        let session = match &self.imap_config.auth {
            ImapAuthConfig::Passwd(_) => build_session(&self.imap_config, creds).await,
            ImapAuthConfig::OAuth2(oauth2_config) => {
                match build_session(&self.imap_config, creds).await {
                    Ok(sess) => Ok(sess),
                    Err(err) => match err {
                        crate::Error::ImapError(Error::AuthenticateError(imap::Error::Parse(
                            imap::error::ParseError::Authentication(_, _),
                        ))) => {
                            warn!("error while authenticating user, refreshing access token");
                            oauth2_config.refresh_access_token().await?;
                            build_session(&self.imap_config, creds).await
                        }
                        err => Err(err),
                    },
                }
            }
        }?;

        Ok(ImapSessionSync {
            account_config: self.account_config.clone(),
            imap_config: self.imap_config.clone(),
            session: Arc::new(Mutex::new(ImapSession {
                account_config: self.account_config,
                imap_config: self.imap_config,
                default_credentials: self.default_credentials,
                session,
            })),
        })
    }
}

/// The IMAP session.
///
/// This session is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`ImapSessionSync`].
#[derive(Debug)]
pub struct ImapSession {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The IMAP configuration.
    pub imap_config: ImapConfig,

    /// The default IMAP credentials.
    pub default_credentials: Option<String>,

    /// The current IMAP session.
    session: Session<ImapSessionStream>,
}

impl ImapSession {
    /// Execute the given action on the current IMAP session.
    ///
    /// If an OAuth 2.0 authentication error occurs, the access token
    /// is refreshed and the action is executed once again.
    pub async fn execute<T>(
        &mut self,
        action: impl Fn(&mut Session<ImapSessionStream>) -> imap::Result<T>,
        map_err: impl Fn(imap::Error) -> Box<dyn std::error::Error + Send>,
    ) -> Result<T> {
        match &self.imap_config.auth {
            ImapAuthConfig::Passwd(_) => Ok(action(&mut self.session)
                .map_err(map_err)
                .map_err(Error::ExecuteSessionActionError)?),
            ImapAuthConfig::OAuth2(oauth2_config) => match action(&mut self.session) {
                Ok(res) => Ok(res),
                Err(err) => match err {
                    imap::Error::Parse(imap::error::ParseError::Authentication(_, _)) => {
                        warn!("error while authenticating user, refreshing access token");
                        oauth2_config.refresh_access_token().await?;
                        let creds = self.default_credentials.as_ref();
                        self.session = build_session(&self.imap_config, creds).await?;
                        Ok(action(&mut self.session)
                            .map_err(map_err)
                            .map_err(Error::ExecuteSessionActionError)?)
                    }
                    err => Ok(Err(Error::ExecuteSessionActionError(Box::new(err)))?),
                },
            },
        }
    }
}

impl Drop for ImapSession {
    fn drop(&mut self) {
        if let Err(err) = self.session.close() {
            warn!("cannot close imap session: {err}");
            debug!("cannot close imap session: {err:?}");
        }
        if let Err(err) = self.session.logout() {
            warn!("cannot logout imap session: {err}");
            debug!("cannot logout imap session: {err:?}");
        }
    }
}

/// The sync version of the IMAP session.
///
/// This is just an IMAP session wrapped into a mutex, so the same
/// IMAP session can be shared and updated across multiple threads.
#[derive(Clone, Debug)]
pub struct ImapSessionSync {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The IMAP configuration.
    pub imap_config: ImapConfig,

    /// The IMAP session wrapped into a mutex.
    session: Arc<Mutex<ImapSession>>,
}

impl ImapSessionSync {
    /// Create a new IMAP sync session from an IMAP session.
    pub fn new(
        account_config: AccountConfig,
        imap_config: ImapConfig,
        session: ImapSession,
    ) -> Self {
        Self {
            account_config,
            imap_config,
            session: Arc::new(Mutex::new(session)),
        }
    }
}

impl Deref for ImapSessionSync {
    type Target = Mutex<ImapSession>;

    fn deref(&self) -> &Self::Target {
        &self.session
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
) -> Result<Session<ImapSessionStream>> {
    let mut session = match &imap_config.auth {
        ImapAuthConfig::Passwd(passwd) => {
            debug!("creating session using login and password");
            let passwd = match credentials {
                Some(passwd) => passwd.to_string(),
                None => passwd
                    .get()
                    .await
                    .map_err(Error::GetPasswdError)?
                    .lines()
                    .next()
                    .ok_or_else(|| Error::GetPasswdEmptyError)?
                    .to_owned(),
            };
            build_client(imap_config)?
                .login(&imap_config.login, passwd)
                .map_err(|res| Error::LoginError(res.0))
        }
        ImapAuthConfig::OAuth2(oauth2_config) => {
            let access_token = match credentials {
                Some(access_token) => access_token.to_string(),
                None => oauth2_config.access_token().await?,
            };
            match oauth2_config.method {
                OAuth2Method::XOAuth2 => {
                    debug!("creating session using xoauth2");
                    let xoauth2 = XOAuth2::new(imap_config.login.clone(), access_token);
                    build_client(imap_config)?
                        .authenticate("XOAUTH2", &xoauth2)
                        .map_err(|(err, _client)| Error::AuthenticateError(err))
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
                        .map_err(|(err, _client)| Error::AuthenticateError(err))
                }
            }
        }
    }?;

    session.debug = log_enabled!(Level::Trace);

    Ok(session)
}

/// Creates a client from an IMAP configuration.
fn build_client(imap_config: &ImapConfig) -> Result<Client<ImapSessionStream>> {
    let mut client_builder = imap::ClientBuilder::new(&imap_config.host, imap_config.port);

    if imap_config.starttls() {
        client_builder.starttls();
    }

    let client = if imap_config.ssl() {
        client_builder.connect(tls_handshake(imap_config)?)
    } else {
        client_builder.connect(tcp_handshake()?)
    }
    .map_err(Error::ConnectError)?;

    Ok(client)
}

/// TCP handshake.
fn tcp_handshake() -> Result<Box<dyn FnOnce(&str, TcpStream) -> imap::Result<ImapSessionStream>>> {
    Ok(Box::new(|_domain, tcp| Ok(ImapSessionStream::Tcp(tcp))))
}

/// TLS/SSL handshake.
fn tls_handshake(
    imap_config: &ImapConfig,
) -> Result<Box<dyn FnOnce(&str, TcpStream) -> imap::Result<ImapSessionStream>>> {
    use rustls::client::WebPkiVerifier;

    struct DummyCertVerifier;
    impl ServerCertVerifier for DummyCertVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &Certificate,
            _intermediates: &[Certificate],
            _server_name: &rustls::ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: std::time::SystemTime,
        ) -> result::Result<rustls::client::ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn request_scts(&self) -> bool {
            false
        }
    }

    let tlsconfig = ClientConfig::builder().with_safe_defaults();

    let tlsconfig = if imap_config.insecure() {
        tlsconfig.with_custom_certificate_verifier(Arc::new(DummyCertVerifier))
    } else {
        let verifier = WebPkiVerifier::new(ROOT_CERT_STORE.clone(), None);
        tlsconfig.with_custom_certificate_verifier(Arc::new(verifier))
    }
    .with_no_client_auth();

    let tlsconfig = Arc::new(tlsconfig);

    Ok(Box::new(|domain, tcp| {
        let name = rustls::ServerName::try_from(domain).map_err(|err| {
            imap::Error::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid domain name ({:?}): {}", err, domain),
            ))
        })?;
        let connection = ClientConnection::new(tlsconfig, name)
            .map_err(|err| io::Error::new(io::ErrorKind::ConnectionAborted, err))?;
        let stream = StreamOwned::new(connection, tcp);
        Ok(ImapSessionStream::Tls(stream))
    }))
}
