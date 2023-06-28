//! Module dedicated to the IMAP backend.
//!
//! This module contains the implementation of the IMAP backend and
//! all associated structures related to it.

pub mod config;

use imap::{
    extensions::{
        idle::{stop_on_any, SetReadTimeout},
        sort::SortCharset,
    },
    Authenticator, Client, Session,
};
use imap_proto::{NameAttribute, UidSetMember};
use log::{debug, error, info, log_enabled, trace, warn, Level};
use once_cell::sync::Lazy;
use pimalaya_process::Cmd;
use rustls::{
    client::{ServerCertVerified, ServerCertVerifier},
    Certificate, ClientConfig, ClientConnection, RootCertStore, StreamOwned,
};
use std::{
    any::Any,
    collections::HashSet,
    io::{self, Read, Write},
    net::TcpStream,
    result, string,
    time::Duration,
};
use thiserror::Error;
use utf7_imap::{decode_utf7_imap as decode_utf7, encode_utf7_imap as encode_utf7};

use crate::{
    envelope, AccountConfig, Backend, Envelope, Envelopes, Flag, Flags, Folder, Folders, Messages,
    OAuth2Method, Result,
};

pub use self::config::{ImapAuthConfig, ImapConfig};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create imap backend: imap not initialized")]
    InitError,

    // Folders
    #[error("cannot create imap folder {1}")]
    CreateFolderError(#[source] imap::Error, String),
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot list imap folders")]
    ListFoldersError(#[source] imap::Error),
    #[error("cannot examine folder {1}")]
    ExamineFolderError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderError(#[source] imap::Error, String),
    #[error("cannot delete imap folder {1}")]
    DeleteFolderError(#[source] imap::Error, String),
    #[error("cannot parse headers of imap email {0}")]
    ParseHeadersOfFetchError(String),

    // Envelopes
    #[error("cannot get imap envelope of email {0}")]
    GetEnvelopeError(String),
    #[error("cannot list imap envelopes: page {0} out of bounds")]
    BuildPageRangeOutOfBoundsError(usize),
    #[error("cannot fetch new imap envelopes")]
    FetchNewEnvelopesError(#[source] imap::Error),
    #[error("cannot search new imap envelopes")]
    SearchNewEnvelopesError(#[source] imap::Error),
    #[error("cannot search imap envelopes in folder {1} with query: {2}")]
    SearchEnvelopesError(#[source] imap::Error, String, String),
    #[error("cannot sort imap envelopes in folder {1} with query: {2}")]
    SortEnvelopesError(#[source] imap::Error, String, String),
    #[error("cannot get next imap envelope uid of folder {0}")]
    GetNextEnvelopeUidError(String),
    #[error("cannot parse imap header date {0}")]
    ParseHeaderDateError(String),

    // Flags
    #[error("cannot add flags {1} to imap email(s) {2}")]
    AddFlagsError(#[source] imap::Error, String, String),
    #[error("cannot set flags {1} to emails(s) {2}")]
    SetFlagsError(#[source] imap::Error, String, String),
    #[error("cannot remove flags {1} from email(s) {2}")]
    RemoveFlagsError(#[source] imap::Error, String, String),

    // Emails
    #[error("cannot copy imap email(s) {1} from {2} to {3}")]
    CopyEmailError(#[source] imap::Error, String, String, String),
    #[error("cannot move email(s) {1} from {2} to {3}")]
    MoveEmailError(#[source] imap::Error, String, String, String),
    #[error("cannot fetch imap email {1}")]
    FetchEmailsByUidError(#[source] imap::Error, String),
    #[error("cannot fetch imap emails within uid range {1}")]
    FetchEmailsByUidRangeError(#[source] imap::Error, String),
    #[error("cannot get added email uid from range {0}")]
    GetAddedEmailUidFromRangeError(String),
    #[error("cannot get added email uid (extensions UIDPLUS not enabled on the server?)")]
    GetAddedEmailUidError,
    #[error("cannot append email to folder {1}")]
    AppendEmailError(#[source] imap::Error, String),

    // Parsing/decoding
    #[error("cannot parse sender from imap envelope")]
    ParseSenderFromImapEnvelopeError,
    #[error("cannot decode sender name from imap envelope")]
    DecodeSenderNameFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot decode sender mailbox from imap envelope")]
    DecodeSenderMailboxFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot decode sender host from imap envelope")]
    DecodeSenderHostFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot decode date from imap envelope")]
    DecodeDateFromImapEnvelopeError(rfc2047_decoder::Error),
    #[error("cannot parse imap sort criterion {0}")]
    ParseSortCriterionError(String),
    #[error("cannot decode subject of imap email {1}")]
    DecodeSubjectError(#[source] rfc2047_decoder::Error, String),
    #[error("cannot get imap sender of email {0}")]
    GetSenderError(String),

    // Sessions
    #[error("cannot find session from pool at cursor {0}")]
    FindSessionByCursorError(usize),
    #[error("cannot parse Message-ID of email {0}")]
    ParseMessageIdError(#[source] string::FromUtf8Error, String),
    #[error("cannot lock imap session: {0}")]
    LockSessionError(String),
    #[error("cannot lock imap sessions pool cursor: {0}")]
    LockSessionsPoolCursorError(String),
    #[error("cannot create tls connector")]
    CreateTlsConnectorError(#[source] rustls::Error),
    #[error("cannot connect to imap server")]
    ConnectError(#[source] imap::Error),
    #[error("cannot login to imap server")]
    LoginError(#[source] imap::Error),
    #[error("cannot authenticate to imap server")]
    AuthenticateError(#[source] imap::Error),
    #[error("cannot start the idle mode")]
    StartIdleModeError(#[source] imap::Error),
    #[error("cannot close imap session")]
    CloseError(#[source] imap::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdError(#[source] pimalaya_secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyError,
}

/// The IMAP query needed to retrieve everything we need to build an
/// [envelope]: UID, flags and headers (Message-ID, From, To, Subject,
/// Date).
const ENVELOPE_QUERY: &str =
    "(UID FLAGS BODY.PEEK[HEADER.FIELDS (MESSAGE-ID FROM TO SUBJECT DATE)])";

/// Native certificates store, mostly used by
/// `Backend::tls_handshake()`.
const ROOT_CERT_STORE: Lazy<RootCertStore> = Lazy::new(|| {
    let mut store = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().unwrap() {
        store.add(&Certificate(cert.0)).unwrap();
    }
    store
});

/// Alias for the IMAP session.
pub type ImapSession = Session<ImapSessionStream>;

/// Alias for the TLS/SSL stream, which is basically a
/// [std::net::TcpStream] wrapped by a [rustls::StreamOwned].
pub type TlsStream = StreamOwned<ClientConnection, TcpStream>;

/// Wrapper around TLS/SSL and TCP streams.
///
/// Since [imap::Session] needs a generic stream type, this wrapper is needed to create the session alias [ImapSession].
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

/// The IMAP backend.
pub struct ImapBackend {
    /// The account configuration.
    account_config: AccountConfig,

    /// The IMAP configuration.
    imap_config: ImapConfig,

    /// The current IMAP session.
    session: ImapSession,
}

impl ImapBackend {
    /// Creates a new IMAP backend.
    ///
    /// The IMAP session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    pub fn new(
        account_config: AccountConfig,
        imap_config: ImapConfig,
        default_credentials: Option<String>,
    ) -> Result<ImapBackend> {
        let session = match &imap_config.auth {
            ImapAuthConfig::Passwd(_) => {
                Self::build_session(&imap_config, default_credentials.clone())
            }
            ImapAuthConfig::OAuth2(oauth2_config) => {
                Self::build_session(&imap_config, default_credentials.clone()).or_else(|err| {
                    match err {
                        crate::Error::ImapError(Error::AuthenticateError(imap::Error::Parse(
                            imap::error::ParseError::Authentication(_, _),
                        ))) => {
                            warn!("error while authenticating user, refreshing access token");
                            oauth2_config.refresh_access_token()?;
                            Self::build_session(&imap_config, None)
                        }
                        err => Err(err),
                    }
                })
            }
        }?;

        Ok(Self {
            account_config,
            imap_config,
            session,
        })
    }

    /// Creates a new session from an IMAP configuration and optional
    /// pre-built credentials.
    ///
    /// Pre-built credentials are useful to prevent building them
    /// every time a new session is created. The main use case is for
    /// the synchronization, where multiple sessions can be created in
    /// a row.
    fn build_session(imap_config: &ImapConfig, credentials: Option<String>) -> Result<ImapSession> {
        let mut session = match &imap_config.auth {
            ImapAuthConfig::Passwd(passwd) => {
                debug!("creating session using login and password");
                let passwd = match credentials {
                    Some(passwd) => passwd,
                    None => passwd
                        .get()
                        .map_err(Error::GetPasswdError)?
                        .lines()
                        .next()
                        .ok_or_else(|| Error::GetPasswdEmptyError)?
                        .to_owned(),
                };
                Self::build_client(imap_config)?
                    .login(&imap_config.login, passwd)
                    .map_err(|res| Error::LoginError(res.0))
            }
            ImapAuthConfig::OAuth2(oauth2_config) => {
                let access_token = match credentials {
                    Some(access_token) => access_token,
                    None => oauth2_config.access_token()?,
                };
                match oauth2_config.method {
                    OAuth2Method::XOAuth2 => {
                        debug!("creating session using xoauth2");
                        let xoauth2 = XOAuth2::new(imap_config.login.clone(), access_token);
                        Self::build_client(imap_config)?
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
                        Self::build_client(imap_config)?
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
            client_builder.connect(Self::tls_handshake(imap_config)?)
        } else {
            client_builder.connect(Self::tcp_handshake()?)
        }
        .map_err(Error::ConnectError)?;

        Ok(client)
    }

    /// TCP handshake.
    fn tcp_handshake() -> Result<Box<dyn FnOnce(&str, TcpStream) -> imap::Result<ImapSessionStream>>>
    {
        Ok(Box::new(|_domain, tcp| Ok(ImapSessionStream::Tcp(tcp))))
    }

    /// TLS/SSL handshake.
    fn tls_handshake(
        imap_config: &ImapConfig,
    ) -> Result<Box<dyn FnOnce(&str, TcpStream) -> imap::Result<ImapSessionStream>>> {
        use rustls::client::WebPkiVerifier;
        use std::sync::Arc;

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

    /// Safe wrapper around IMAP session that handles token
    /// refreshing.
    ///
    /// Runs the given action. If an OAuth 2.0 authentication error
    /// occurs, refreshes the tokens then retries once again.
    fn with_session<T>(
        &mut self,
        action: impl Fn(&mut ImapSession) -> imap::Result<T>,
        map_err: impl Fn(imap::Error) -> Error,
    ) -> Result<T> {
        match &self.imap_config.auth {
            ImapAuthConfig::Passwd(_) => {
                action(&mut self.session).or_else(|err| Ok(Err(map_err(err))?))
            }
            ImapAuthConfig::OAuth2(oauth2_config) => {
                action(&mut self.session).or_else(|err| match err {
                    imap::Error::Parse(imap::error::ParseError::Authentication(_, _)) => {
                        warn!("error while authenticating user, refreshing access token");
                        oauth2_config.refresh_access_token()?;
                        self.session = Self::build_session(&self.imap_config, None)?;
                        action(&mut self.session).or_else(|err| Ok(Err(map_err(err))?))
                    }
                    err => Ok(Err(map_err(err))?),
                })
            }
        }
    }

    /// Runs the IMAP notify query in order to get the list of new
    /// envelopes UIDs.
    fn search_new_envelopes(&mut self) -> Result<Vec<u32>> {
        let query = self.imap_config.notify_query();
        let uids: Vec<u32> = self
            .with_session(
                |session| session.uid_search(&query),
                Error::SearchNewEnvelopesError,
            )?
            .into_iter()
            .collect();
        debug!("found {} new envelopes", uids.len());
        trace!("uids: {:?}", uids);

        Ok(uids)
    }

    /// Starts the notify daemon.
    ///
    /// The notify service uses the IDLE IMAP mode to wait for changes
    /// on the server and to run the notify command from the account
    /// configuration in case new envelopes are available.
    pub fn notify(&mut self, keepalive: u64, folder: &str) -> Result<()> {
        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.examine(&folder_encoded),
            |err| Error::ExamineFolderError(err, folder.to_owned()),
        )?;

        debug!("init messages hashset");
        let mut msgs_set: HashSet<u32> = self
            .search_new_envelopes()?
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        trace!("messages hashset: {:?}", msgs_set);

        loop {
            debug!("begin loop");

            self.with_session(
                |session| {
                    session
                        .idle()
                        .timeout(Duration::new(keepalive, 0))
                        .wait_while(stop_on_any)
                },
                Error::StartIdleModeError,
            )?;

            let uids: Vec<u32> = self
                .search_new_envelopes()?
                .into_iter()
                .filter(|uid| msgs_set.get(uid).is_none())
                .collect();
            debug!("found {} new messages not in hashset", uids.len());
            trace!("messages hashet: {:?}", msgs_set);

            if !uids.is_empty() {
                let uids = uids
                    .iter()
                    .map(|uid| uid.to_string())
                    .collect::<Vec<_>>()
                    .join(",");

                let fetches = self.with_session(
                    |session| session.uid_fetch(&uids, ENVELOPE_QUERY),
                    Error::FetchNewEnvelopesError,
                )?;

                for fetch in fetches.iter() {
                    let envelope = Envelope::from(fetch);
                    let uid = fetch.uid.expect("UID should be included in the IMAP fetch");

                    let from = envelope.from.addr.clone();
                    self.imap_config
                        .run_notify_cmd(uid, &envelope.subject, &from)?;

                    debug!("notify message: {}", uid);
                    trace!("message: {:?}", envelope);

                    debug!("insert message {} in hashset", uid);
                    msgs_set.insert(uid);
                    trace!("messages hashset: {:?}", msgs_set);
                }
            }

            debug!("end loop");
        }
    }

    /// Starts the watch daemon.
    ///
    /// The watch service uses the IDLE IMAP mode to wait for changes
    /// on the server and to run the watch commands from the account
    /// configuration, in series.
    pub fn watch(&mut self, keepalive: u64, folder: &str) -> Result<()> {
        debug!("examine folder: {}", folder);

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.examine(&folder_encoded),
            |err| Error::ExamineFolderError(err, folder.to_owned()),
        )?;

        loop {
            debug!("begin loop");

            for (i, cmd) in self.imap_config.watch_cmds().iter().enumerate() {
                debug!("running watch command {}: {cmd}", i + 1);
                match Cmd::from(cmd.clone()).run() {
                    Ok(output) => {
                        debug!("watch command {} successfully executed", i + 1);
                        trace!("exit code: {}", output.code);
                        trace!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                        trace!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                    }
                    Err(err) => {
                        warn!("error while running command {cmd}, skipping it");
                        warn!("{err}")
                    }
                }
            }

            self.with_session(
                |session| {
                    session
                        .idle()
                        .timeout(Duration::new(keepalive, 0))
                        .wait_while(stop_on_any)
                },
                Error::StartIdleModeError,
            )?;

            debug!("end loop");
        }
    }
}

impl Backend for ImapBackend {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    fn add_folder(&mut self, folder: &str) -> Result<()> {
        info!("adding imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.create(&folder_encoded),
            |err| Error::CreateFolderError(err, folder.to_owned()),
        )?;

        Ok(())
    }

    fn list_folders(&mut self) -> Result<Folders> {
        info!("listing imap folders");

        let folders = self.with_session(
            |session| session.list(Some(""), Some("*")),
            Error::ListFoldersError,
        )?;

        let folders = Folders::from_iter(folders.iter().filter_map(|folder| {
            if folder.attributes().contains(&NameAttribute::NoSelect) {
                None
            } else {
                Some(Folder {
                    name: decode_utf7(folder.name().into()),
                    desc: folder
                        .attributes()
                        .iter()
                        .map(|attr| format!("{attr:?}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                })
            }
        }));
        trace!("imap folders: {:?}", folders);

        Ok(folders)
    }

    fn expunge_folder(&mut self, folder: &str) -> Result<()> {
        info!("expunging imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        self.with_session(
            |session| session.expunge(),
            |err| Error::ExpungeFolderError(err, folder.to_owned()),
        )?;

        Ok(())
    }

    fn purge_folder(&mut self, folder: &str) -> Result<()> {
        info!("purging imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let flags = Flags::from_iter([Flag::Deleted]);
        let uids = String::from("1:*");

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        self.with_session(
            |session| {
                session.uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query_string()))
            },
            |err| Error::AddFlagsError(err, flags.to_imap_query_string(), uids.clone()),
        )?;

        self.with_session(
            |session| session.expunge(),
            |err| Error::ExpungeFolderError(err, folder.to_owned()),
        )?;

        Ok(())
    }

    fn delete_folder(&mut self, folder: &str) -> Result<()> {
        info!("deleting imap folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.delete(&folder_encoded),
            |err| Error::DeleteFolderError(err, folder.to_owned()),
        )?;

        Ok(())
    }

    fn get_envelope(&mut self, folder: &str, uid: &str) -> Result<Envelope> {
        info!("getting imap envelope {uid} from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        let fetches = self.with_session(
            |session| session.uid_fetch(uid, ENVELOPE_QUERY),
            |err| Error::FetchEmailsByUidError(err, uid.to_owned()),
        )?;

        let fetch = fetches
            .get(0)
            .ok_or_else(|| Error::GetEnvelopeError(uid.to_owned()))?;

        let envelope = Envelope::from(fetch);
        trace!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }

    fn list_envelopes(&mut self, folder: &str, page_size: usize, page: usize) -> Result<Envelopes> {
        info!("listing imap envelopes from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {folder_encoded}");

        let folder_size = self
            .with_session(
                |session| session.select(&folder_encoded),
                |err| Error::SelectFolderError(err, folder.to_owned()),
            )?
            .exists as usize;
        debug!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let range = build_page_range(page, page_size, folder_size)?;
        debug!("page range: {range}");

        let fetches = self.with_session(
            |session| session.fetch(&range, ENVELOPE_QUERY),
            |err| Error::FetchEmailsByUidRangeError(err, range.clone()),
        )?;

        let envelopes = Envelopes::from(fetches);
        debug!("imap envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        info!("searching imap envelopes from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let folder_size = self
            .with_session(
                |session| session.select(&folder_encoded),
                |err| Error::SelectFolderError(err, folder.to_owned()),
            )?
            .exists as usize;
        trace!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let uids: Vec<String> = if sort.is_empty() {
            self.with_session(
                |session| session.uid_search(query),
                |err| Error::SearchEnvelopesError(err, folder.to_owned(), query.to_owned()),
            )?
            .iter()
            .map(ToString::to_string)
            .collect()
        } else {
            let sort: envelope::imap::SortCriteria = sort.parse()?;
            self.with_session(
                |session| session.uid_sort(&sort, SortCharset::Utf8, query),
                |err| Error::SortEnvelopesError(err, folder.to_owned(), query.to_owned()),
            )?
            .iter()
            .map(ToString::to_string)
            .collect()
        };
        trace!("uids: {uids:?}");

        if uids.is_empty() {
            return Ok(Envelopes::default());
        }

        let uid_range = if page_size > 0 {
            let begin = uids.len().min(page * page_size);
            let end = begin + uids.len().min(page_size);
            if end > begin + 1 {
                uids[begin..end].join(",")
            } else {
                uids[0].to_string()
            }
        } else {
            uids.join(",")
        };
        trace!("page: {page}");
        trace!("page size: {page_size}");
        trace!("uid range: {uid_range}");

        let fetches = self.with_session(
            |session| session.uid_fetch(&uid_range, ENVELOPE_QUERY),
            |err| Error::FetchEmailsByUidRangeError(err, uid_range.clone()),
        )?;

        let envelopes = Envelopes::from(fetches);
        debug!("imap envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String> {
        info!(
            "adding imap email to folder {folder} with flags {flags}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        let appended = self.with_session(
            |session| {
                session
                    .append(&folder, email)
                    .flags(flags.to_imap_flags_vec())
                    .finish()
            },
            |err| Error::AppendEmailError(err, folder.to_owned()),
        )?;

        let uid = match appended.uids {
            Some(mut uids) if uids.len() == 1 => match uids.get_mut(0).unwrap() {
                UidSetMember::Uid(uid) => Ok(*uid),
                UidSetMember::UidRange(uids) => Ok(uids.next().ok_or_else(|| {
                    Error::GetAddedEmailUidFromRangeError(uids.fold(String::new(), |range, uid| {
                        if range.is_empty() {
                            uid.to_string()
                        } else {
                            range + ", " + &uid.to_string()
                        }
                    }))
                })?),
            },
            _ => {
                // TODO: find a way to retrieve the UID of the added
                // email (by Message-ID?)
                Err(Error::GetAddedEmailUidError)
            }
        }?;
        trace!("uid: {uid}");

        Ok(uid.to_string())
    }

    fn preview_emails(&mut self, folder: &str, uids: Vec<&str>) -> Result<Messages> {
        let uids = uids.join(",");
        info!("previewing imap emails {uids} from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        let fetches = self.with_session(
            |session| session.uid_fetch(&uids, "BODY.PEEK[]"),
            |err| Error::FetchEmailsByUidRangeError(err, uids.clone()),
        )?;

        Ok(Messages::try_from(fetches)?)
    }

    fn get_emails(&mut self, folder: &str, uids: Vec<&str>) -> Result<Messages> {
        let uids = uids.join(",");
        info!("getting imap emails {uids} from folder {folder}");

        let folder_encoded = encode_utf7(folder.to_owned());
        trace!("utf7 encoded folder: {folder_encoded}");

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        let fetches = self.with_session(
            |session| session.uid_fetch(&uids, "BODY[]"),
            |err| Error::FetchEmailsByUidRangeError(err, uids.clone()),
        )?;

        Ok(Messages::try_from(fetches)?)
    }

    fn copy_emails(&mut self, from_folder: &str, to_folder: &str, uids: Vec<&str>) -> Result<()> {
        let uids = uids.join(",");
        info!("copying imap emails {uids} from folder {from_folder} to folder {to_folder}");

        let from_folder_encoded = encode_utf7(from_folder.to_owned());
        let to_folder_encoded = encode_utf7(to_folder.to_owned());
        trace!("utf7 encoded from folder: {}", from_folder_encoded);
        trace!("utf7 encoded to folder: {}", to_folder_encoded);

        self.with_session(
            |session| session.select(&from_folder_encoded),
            |err| Error::SelectFolderError(err, from_folder.to_owned()),
        )?;

        self.with_session(
            |session| session.uid_copy(&uids, &to_folder_encoded),
            |err| {
                Error::CopyEmailError(
                    err,
                    uids.clone(),
                    from_folder.to_owned(),
                    to_folder.to_owned(),
                )
            },
        )?;

        Ok(())
    }

    fn move_emails(&mut self, from_folder: &str, to_folder: &str, uids: Vec<&str>) -> Result<()> {
        let uids = uids.join(",");
        info!("moving imap emails {uids} from folder {from_folder} to folder {to_folder}");

        let from_folder_encoded = encode_utf7(from_folder.to_owned());
        let to_folder_encoded = encode_utf7(to_folder.to_owned());
        trace!("utf7 encoded from folder: {}", from_folder_encoded);
        trace!("utf7 encoded to folder: {}", to_folder_encoded);

        self.with_session(
            |session| session.select(&from_folder_encoded),
            |err| Error::SelectFolderError(err, from_folder.to_owned()),
        )?;

        self.with_session(
            |session| session.uid_mv(&uids, &to_folder_encoded),
            |err| {
                Error::MoveEmailError(
                    err,
                    uids.clone(),
                    from_folder.to_owned(),
                    to_folder.to_owned(),
                )
            },
        )?;

        Ok(())
    }

    fn delete_emails(&mut self, folder: &str, uids: Vec<&str>) -> Result<()> {
        let trash_folder = self.account_config.trash_folder_alias()?;

        if self.account_config.folder_alias(folder)? == trash_folder {
            self.mark_emails_as_deleted(folder, uids)
        } else {
            self.move_emails(folder, &trash_folder, uids)
        }
    }

    fn add_flags(&mut self, folder: &str, uids: Vec<&str>, flags: &Flags) -> Result<()> {
        let uids = uids.join(",");
        info!(
            "addings flags {flags} to imap emails {uids} from folder {folder}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {}", folder_encoded);

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        self.with_session(
            |session| {
                session.uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query_string()))
            },
            |err| Error::AddFlagsError(err, flags.to_imap_query_string(), uids.clone()),
        )?;

        Ok(())
    }

    fn set_flags(&mut self, folder: &str, uids: Vec<&str>, flags: &Flags) -> Result<()> {
        let uids = uids.join(",");
        info!(
            "setting flags {flags} to imap emails {uids} from folder {folder}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {}", folder_encoded);

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        self.with_session(
            |session| session.uid_store(&uids, format!("FLAGS ({})", flags.to_imap_query_string())),
            |err| Error::SetFlagsError(err, flags.to_imap_query_string(), uids.clone()),
        )?;

        Ok(())
    }

    fn remove_flags(&mut self, folder: &str, uids: Vec<&str>, flags: &Flags) -> Result<()> {
        let uids = uids.join(",");
        info!(
            "removing flags {flags} to imap emails {uids} from folder {folder}",
            flags = flags.to_string(),
        );

        let folder_encoded = encode_utf7(folder.to_owned());
        debug!("utf7 encoded folder: {}", folder_encoded);

        self.with_session(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.to_owned()),
        )?;

        self.with_session(
            |session| {
                session.uid_store(&uids, format!("-FLAGS ({})", flags.to_imap_query_string()))
            },
            |err| Error::RemoveFlagsError(err, flags.to_imap_query_string(), uids.clone()),
        )?;

        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        debug!("closing imap backend session");
        self.with_session(
            |session| {
                session
                    .check()
                    .and_then(|()| session.close())
                    .or(imap::Result::Ok(()))
            },
            Error::CloseError,
        )?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for ImapBackend {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            warn!("cannot close imap session, skipping it: {err}");
            error!("cannot close imap session: {err:?}");
        }
    }
}

/// Builds the IMAP sequence set for the give page, page size and
/// total size.
pub fn build_page_range(page: usize, page_size: usize, size: usize) -> Result<String> {
    let page_cursor = page * page_size;
    if page_cursor >= size {
        return Err(Error::BuildPageRangeOutOfBoundsError(page + 1))?;
    }

    let range = if page_size == 0 {
        String::from("1:*")
    } else {
        let page_size = page_size.min(size);
        let mut count = 1;
        let mut cursor = size - (size.min(page_cursor));
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
            crate::Error::ImapError(super::Error::BuildPageRangeOutOfBoundsError(2)),
        ));

        // page * page_size > size
        assert!(matches!(
            super::build_page_range(2, 5, 5).unwrap_err(),
            crate::Error::ImapError(super::Error::BuildPageRangeOutOfBoundsError(3)),
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
