use directory::core::config::ConfigDirectory;
use imap_stalwart::core::{ImapSessionManager, IMAP};
use jmap::{api::JmapSessionManager, services::IPC_CHANNEL_BUFFER, JMAP};
use managesieve::core::ManageSieveSessionManager;
use smtp::core::{SmtpSessionManager, SMTP};
use std::time::Duration;
use store::config::ConfigStore;
use tokio::sync::mpsc;
use utils::{
    config::{Config, ServerProtocol},
    UnwrapFailure,
};

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::test(flavor = "multi_thread")]
async fn stalwart() -> std::io::Result<()> {
    let mut config = Config::new(include_str!("./stalwart.toml")).unwrap();

    // // Enable tracing
    // let _tracer = enable_tracing(
    //     &config,
    //     &format!(
    //         "Starting Stalwart Mail Server v{}...",
    //         env!("CARGO_PKG_VERSION"),
    //     ),
    // )
    // .failed("Failed to enable tracing");

    // Bind ports and drop privileges
    let mut servers = config.parse_servers().failed("Invalid configuration");
    servers.bind(&config);

    // Parse stores
    let stores = config.parse_stores().await.failed("Invalid configuration");
    let data_store = stores
        .get_store(&config, "storage.data")
        .failed("Invalid configuration");

    // Update configuration
    config.update(data_store.config_list("").await.failed("Storage error"));

    // Parse directories
    let directory = config
        .parse_directory(&stores, data_store)
        .await
        .failed("Invalid configuration");
    let schedulers = config
        .parse_purge_schedules(
            &stores,
            config.value("storage.data"),
            config.value("storage.blob"),
        )
        .await
        .failed("Invalid configuration");

    // Init servers
    let (delivery_tx, delivery_rx) = mpsc::channel(IPC_CHANNEL_BUFFER);
    let smtp = SMTP::init(&config, &servers, &stores, &directory, delivery_tx)
        .await
        .failed("Invalid configuration file");
    let jmap = JMAP::init(
        &config,
        &stores,
        &directory,
        &mut servers,
        delivery_rx,
        smtp.clone(),
    )
    .await
    .failed("Invalid configuration file");
    let imap = IMAP::init(&config)
        .await
        .failed("Invalid configuration file");
    jmap.directory
        .blocked_ips
        .reload(&config)
        .failed("Invalid configuration");

    // Spawn servers
    let (shutdown_tx, shutdown_rx) = servers.spawn(|server, shutdown_rx| {
        match &server.protocol {
            ServerProtocol::Smtp | ServerProtocol::Lmtp => {
                server.spawn(SmtpSessionManager::new(smtp.clone()), shutdown_rx)
            }
            ServerProtocol::Http => {
                // tracing::debug!("Ignoring HTTP server listener, using JMAP port instead.");
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

    // // Wait for shutdown signal
    // wait_for_shutdown(&format!(
    //     "Shutting down Stalwart Mail Server v{}...",
    //     env!("CARGO_PKG_VERSION")
    // ))
    // .await;

    // Stop services
    let _ = shutdown_tx.send(true);

    // Wait for services to finish
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
