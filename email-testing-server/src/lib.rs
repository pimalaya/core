use directory::core::config::ConfigDirectory;
use imap::core::{ImapSessionManager, IMAP};
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;
use jmap::{api::JmapSessionManager, services::IPC_CHANNEL_BUFFER, JMAP};
use managesieve::core::ManageSieveSessionManager;
use smtp::core::{SmtpSessionManager, SMTP};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    net::TcpListener,
    time::Duration,
};
use store::config::ConfigStore;
use tempfile::tempdir;
use tokio::sync::mpsc;
use utils::config::{Config, ServerProtocol};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/// Spawn a JMAP, IMAP and SMTP servers for testing purpose. Ports are
/// randomly generated, so multiple servers can be spawned at the same
/// time.
///
/// The code is heavily inspired from the [`main.rs`] of stalwartlabs/mail-server.
///
/// [`main.rs`]: https://github.com/stalwartlabs/mail-server/blob/main/crates/main/src/main.rs
pub async fn with_email_testing_server<F: Future<Output = ()>>(task: impl Fn(Ports) -> F) {
    let tmp = tempdir().expect("should create a temporary directory");
    let tmp = tmp.path();

    let sqlite_path = tmp.join("stalwart.sqlite").to_string_lossy().to_string();

    let ports = Ports::new();
    let imap_bind = format!("[::]:{}", ports.imap);
    let smtp_bind = format!("[::]:{}", ports.smtp);

    let mut config = Config {
        keys: BTreeMap::from_iter([
            ("server.hostname".into(), "localhost".into()),
            ("server.listener.imap.protocol".into(), "imap".into()),
            ("server.listener.imap.bind.0000".into(), imap_bind),
            ("server.listener.smtp.protocol".into(), "smtp".into()),
            ("server.listener.smtp.bind.0000".into(), smtp_bind),
            ("directory.memory.disable".into(), "false".into()),
            ("directory.memory.type".into(), "memory".into()),
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
    let schedulers = config
        .parse_purge_schedules(
            &stores,
            config.value("storage.data"),
            config.value("storage.blob"),
        )
        .await
        .expect("schedulers config should be valid");

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

    jmap.directory
        .blocked_ips
        .reload(&config)
        .expect("should be able to set up JMAP blocked ips");

    // Spawn servers
    let (shutdown_tx, shutdown_rx) = servers.spawn(|server, shutdown_rx| {
        match &server.protocol {
            ServerProtocol::Smtp | ServerProtocol::Lmtp => {
                server.spawn(SmtpSessionManager::new(smtp.clone()), shutdown_rx)
            }
            ServerProtocol::Http => {
                unreachable!();
            }
            ServerProtocol::Jmap => {
                server.spawn(JmapSessionManager::new(jmap.clone()), shutdown_rx)
            }
            ServerProtocol::Imap => server.spawn(
                ImapSessionManager::new(jmap.clone(), imap.clone()),
                shutdown_rx,
            ),
            ServerProtocol::ManageSieve => server.spawn(
                ManageSieveSessionManager::new(jmap.clone(), imap.clone()),
                shutdown_rx,
            ),
        };
    });

    // Spawn purge schedulers
    for scheduler in schedulers {
        scheduler.spawn(shutdown_rx.clone());
    }

    // Execute the task
    task(ports).await;

    // Stop services
    shutdown_tx
        .send(true)
        .expect("should send shutdown message to servers");

    // Wait for services to finish
    tokio::time::sleep(Duration::from_secs(1)).await;
}

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
