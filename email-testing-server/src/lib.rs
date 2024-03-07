use directory::core::config::ConfigDirectory;
use imap::core::{ImapSessionManager, IMAP};
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use jmap::{services::IPC_CHANNEL_BUFFER, JMAP};
use log::{log_enabled, Level::*};
use smtp::core::{SmtpSessionManager, SMTP};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    net::TcpListener,
};
use store::config::ConfigStore;
use tempfile::tempdir;
use tokio::sync::mpsc;
use utils::{
    config::{Config, ServerProtocol},
    enable_tracing,
};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub async fn start_email_testing_server() -> (Ports, impl Fn()) {
    // NOTE: did not find a way to get the current log level
    let tracing_level = if log_enabled!(Trace) {
        "trace"
    } else if log_enabled!(Debug) {
        "debug"
    } else if log_enabled!(Info) {
        "info"
    } else if log_enabled!(Warn) {
        "warn"
    } else {
        "error"
    };

    let tmp = tempdir().expect("should create a temporary directory");
    let tmp = tmp.path();
    let sqlite_path = tmp.join("database.sqlite").to_string_lossy().to_string();

    let ports = Ports::new();
    let imap_bind = format!("[::]:{}", ports.imap);
    let smtp_bind = format!("[::]:{}", ports.smtp);

    let mut config = Config {
        keys: BTreeMap::from_iter([
            ("global.tracing.method".into(), "stdout".into()),
            ("global.tracing.level".into(), tracing_level.into()),
            ("server.hostname".into(), "localhost".into()),
            ("server.tls.enable".into(), "false".into()),
            ("server.listener.imap.protocol".into(), "imap".into()),
            ("server.listener.imap.bind.0000".into(), imap_bind),
            ("server.listener.smtp.protocol".into(), "smtp".into()),
            ("server.listener.smtp.bind.0000".into(), smtp_bind),
            ("imap.auth.allow-plain-text".into(), "true".into()),
            ("imap.protocol.uidplus".into(), "true".into()),
            ("imap.rate-limit.concurrent".into(), "32".into()),
            ("imap.rate-limit.requests".into(), "100000/1s".into()),
            ("queue.outbound.next-hop".into(), "'local'".into()),
            ("session.ehlo.reject-non-fqdn".into(), "false".into()),
            ("session.auth.mechanisms".into(), "[plain]".into()),
            ("session.auth.directory".into(), "'memory'".into()),
            ("session.auth.allow-plain-text".into(), "true".into()),
            ("session.rcpt.relay".into(), "true".into()),
            ("session.rcpt.directory".into(), "'memory'".into()),
            ("directory.memory.type".into(), "memory".into()),
            ("directory.memory.options.catch-all".into(), "true".into()),
            ("directory.memory.disable".into(), "false".into()),
            (
                "directory.memory.principals.0.type".into(),
                "individual".into(),
            ),
            ("directory.memory.principals.0.name".into(), "alice".into()),
            (
                "directory.memory.principals.0.secret".into(),
                "password".into(),
            ),
            (
                "directory.memory.principals.0.email.0".into(),
                "alice@localhost".into(),
            ),
            (
                "directory.memory.principals.1.type".into(),
                "individual".into(),
            ),
            ("directory.memory.principals.1.name".into(), "bob".into()),
            (
                "directory.memory.principals.1.secret".into(),
                "password".into(),
            ),
            (
                "directory.memory.principals.1.email.1".into(),
                "bob@localhost".into(),
            ),
            ("storage.data".into(), "sqlite".into()),
            ("storage.blob".into(), "sqlite".into()),
            ("storage.fts".into(), "sqlite".into()),
            ("storage.lookup".into(), "sqlite".into()),
            ("storage.directory".into(), "memory".into()),
            ("store.sqlite.type".into(), "sqlite".into()),
            ("store.sqlite.disable".into(), "false".into()),
            ("store.sqlite.path".into(), sqlite_path),
            ("resolver.type".into(), "system".into()),
        ]),
    };

    // Enable tracing
    enable_tracing(
        &config,
        &format!(
            "Starting Stalwart Mail Server v{}...",
            env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("should enable tracing");

    // Bind ports and drop privileges
    let mut servers = config
        .parse_servers()
        .expect("servers config should be valid");
    servers.bind(&config);

    // Parse stores
    let stores = config
        .parse_stores()
        .await
        .expect("stores config should be valid");
    let data_store = stores
        .get_store(&config, "storage.data")
        .expect("data stores config should be valid");

    // Update configuration
    config.update(
        data_store
            .config_list("")
            .await
            .expect("should be able to save data store config"),
    );

    // Parse directories
    let directory = config
        .parse_directory(&stores, data_store)
        .await
        .expect("directory config should be valid");

    // Init servers
    let (delivery_tx, delivery_rx) = mpsc::channel(IPC_CHANNEL_BUFFER);

    let smtp = SMTP::init(&config, &servers, &stores, &directory, delivery_tx)
        .await
        .expect("should be able to init SMTP server");

    let jmap = JMAP::init(
        &config,
        &stores,
        &directory,
        &mut servers,
        delivery_rx,
        smtp.clone(),
    )
    .await
    .expect("should be able to init JMAP server");

    let imap = IMAP::init(&config)
        .await
        .expect("should be able to init IMAP server");

    // Spawn servers
    let (shutdown_tx, _) = servers.spawn(|server, shutdown_rx| {
        match &server.protocol {
            ServerProtocol::Smtp | ServerProtocol::Lmtp => {
                server.spawn(SmtpSessionManager::new(smtp.clone()), shutdown_rx)
            }
            ServerProtocol::Http => {
                unreachable!();
            }
            ServerProtocol::Jmap => {
                unreachable!();
                // server.spawn(JmapSessionManager::new(jmap.clone()), shutdown_rx)
            }
            ServerProtocol::Imap => server.spawn(
                ImapSessionManager::new(jmap.clone(), imap.clone()),
                shutdown_rx,
            ),
            ServerProtocol::ManageSieve => {
                unreachable!();
                // server.spawn(
                //     ManageSieveSessionManager::new(jmap.clone(), imap.clone()),
                //     shutdown_rx,
                // )
            }
        };
    });

    let shutdown = move || {
        shutdown_tx
            .send(true)
            .expect("should send shutdown message to servers")
    };

    (ports, shutdown)
}

/// Spawn a JMAP, IMAP and SMTP servers for testing purpose. Ports are
/// randomly generated, so multiple servers can be spawned at the same
/// time.
///
/// The code is heavily inspired from the [`main.rs`] of stalwartlabs/mail-server.
///
/// [`main.rs`]: https://github.com/stalwartlabs/mail-server/blob/main/crates/main/src/main.rs
pub async fn with_email_testing_server<F: Future<Output = ()> + Send>(
    task: impl Fn(Ports) -> F + Send + Sync + 'static,
) {
    let (ports, shutdown) = start_email_testing_server().await;
    task(ports).await;
    shutdown();
}

#[derive(Clone, Debug)]
pub struct Ports {
    pub imap: u16,
    pub jmap: u16,
    pub smtp: u16,
}

impl Ports {
    fn new() -> Self {
        Self {
            imap: Self::get_first_random_available_port(),
            jmap: Self::get_first_random_available_port(),
            smtp: Self::get_first_random_available_port(),
        }
    }

    fn get_first_random_available_port() -> u16 {
        (49_152..65_535)
            .collect::<HashSet<u16>>()
            .into_iter()
            .find(|port| TcpListener::bind(("localhost", *port)).is_ok())
            .expect("should find a free port")
    }
}
