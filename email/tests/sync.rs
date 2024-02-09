use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    flag::{Flag, Flags},
    folder::config::FolderConfig,
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    sync::{SyncBuilder, SyncEvent},
};
use env_logger;
use mail_builder::MessageBuilder;
use once_cell::sync::Lazy;
use secret::Secret;
use std::{collections::HashMap, collections::HashSet, sync::Arc, time::Duration};
use tempfile::tempdir;
use tokio::sync::Mutex;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync() {
    env_logger::builder().is_test(true).init();

    let tmp = tempdir().unwrap().path().to_owned();

    // set up left configs

    let mdir_config_left = Arc::new(MaildirConfig {
        root_dir: tmp.join("left"),
    });

    let account_config_left = Arc::new(AccountConfig {
        name: "left".into(),
        ..Default::default()
    });

    // set up right configs

    let imap_config_right = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: 3143,
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..ImapConfig::default()
    });

    let account_config_right = Arc::new(AccountConfig {
        name: "right".into(),
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([(
                "inbox".into(),
                "custom-inbox".into(),
            )])),
            ..Default::default()
        }),
        ..Default::default()
    });

    // set up left backends

    let left_ctx = MaildirContextBuilder::new(mdir_config_left);
    let left_builder = BackendBuilder::new(account_config_left.clone(), left_ctx);

    // set up right backends

    let right_ctx = ImapContextBuilder::new(imap_config_right.clone());
    let right_builder = BackendBuilder::new(account_config_right.clone(), right_ctx);
    let right = right_builder.clone().build().await.unwrap();

    for folder in right.list_folders().await.unwrap().iter() {
        let _ = right.delete_folder(&folder.name).await;
    }

    right.add_folder("inbox").await.unwrap();
    right.add_folder("sync").await.unwrap();

    // add three emails to folder INBOX with delay (in order to have
    // different dates)

    right
        .add_message_with_flag(
            "inbox",
            &MessageBuilder::new()
                .message_id("<a@localhost>")
                .from("alice@localhost")
                .to("bob@localhost")
                .subject("A")
                .text_body("A")
                .write_to_vec()
                .unwrap(),
            Flag::Seen,
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    right
        .add_message_with_flags(
            "inbox",
            &MessageBuilder::new()
                .message_id("<b@localhost>")
                .from("alice@localhost")
                .to("bob@localhost")
                .subject("B")
                .text_body("B")
                .write_to_vec()
                .unwrap(),
            &Flags::from_iter([Flag::Seen, Flag::Flagged, Flag::Custom("custom".into())]),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    right
        .add_message(
            "inbox",
            &MessageBuilder::new()
                .message_id("<c@localhost>")
                .from("alice@localhost")
                .to("bob@localhost")
                .subject("C")
                .text_body("C")
                .write_to_vec()
                .unwrap(),
        )
        .await
        .unwrap();

    // add two more emails to folder [Gmail]/Sent

    // right
    //     .add_message_with_flags(
    //         "sent",
    //         &MessageBuilder::new()
    //             .message_id("<d@localhost>")
    //             .from("alice@localhost")
    //             .to("bob@localhost")
    //             .subject("D")
    //             .text_body("D")
    //             .write_to_vec()
    //             .unwrap(),
    //         &Flags::default(),
    //     )
    //     .await
    //     .unwrap();

    // tokio::time::sleep(Duration::from_secs(1)).await;

    // right
    //     .add_message_with_flags(
    //         "SenT",
    //         &MessageBuilder::new()
    //             .message_id("<e@localhost>")
    //             .from("alice@localhost")
    //             .to("bob@localhost")
    //             .subject("E")
    //             .text_body("E")
    //             .write_to_vec()
    //             .unwrap(),
    //         &Flags::default(),
    //     )
    //     .await
    //     .unwrap();

    // sync imap account twice in a row to see if all work as expected
    // without duplicate items

    static EVENTS_STACK: Lazy<Mutex<HashSet<SyncEvent>>> =
        Lazy::new(|| Mutex::const_new(HashSet::default()));

    let sync_builder = SyncBuilder::new(left_builder, right_builder)
        .with_cache_dir(tmp.join("cache"))
        .with_handler(|evt| async {
            let mut stack = EVENTS_STACK.lock().await;
            stack.insert(evt);
            Ok(())
        });

    let report = sync_builder.sync().await.unwrap();
    println!("report: {:#?}", report);

    let expected_folders = HashSet::from_iter(["INBOX".into(), "sync".into()]);
    assert_eq!(report.folder.folders, expected_folders)

    // let evts = EVENTS_STACK.lock().await;
    // let expected_evts = HashSet::from_iter([
    //     FolderSyncEvent::ListedLeftCachedFolders(1),
    //     FolderSyncEvent::ListedRightCachedFolders(1),
    //     FolderSyncEvent::ListedLeftFolders(1),
    //     FolderSyncEvent::ListedRightFolders(2),
    //     FolderSyncEvent::ListedAllFolders,
    //     FolderSyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
    //         "sync".into(),
    //         SyncDestination::Right,
    //     )),
    //     FolderSyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(
    //         "sync".into(),
    //         SyncDestination::Left,
    //     )),
    //     FolderSyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
    //         "sync".into(),
    //         SyncDestination::Left,
    //     )),
    // ]);

    // assert_eq!(*evts, expected_evts);

    // let folder_patch: Vec<_> = report
    //     .folder
    //     .patch
    //     .into_iter()
    //     .map(|(hunk, _err)| hunk)
    //     .collect();
    // let expected_folder_patch: Vec<FolderSyncHunk> = vec![
    //     FolderSyncHunk::Cache("sync".into(), SyncDestination::Right),
    //     FolderSyncHunk::Create("sync".into(), SyncDestination::Left),
    //     FolderSyncHunk::Cache("sync".into(), SyncDestination::Left),
    // ];

    // assert_eq!(folder_patch, expected_folder_patch);
}
