use email::maildir::MaildirContextBuilder;
use email::{
    account::{
        config::{passwd::PasswdConfig, AccountConfig},
        sync::{config::SyncConfig, AccountSyncBuilder},
    },
    backend::BackendBuilder,
    email::sync::EmailSyncCache,
    envelope::Id,
    flag::{Flag, Flags},
    folder::{self, config::FolderConfig, FolderKind, INBOX, SENT, TRASH},
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
    maildir::config::MaildirConfig,
};
use env_logger;
use mail_builder::MessageBuilder;
use secret::Secret;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread")]
async fn sync() {
    env_logger::builder().is_test(true).init();

    // set up config

    let sync_dir = tempdir().unwrap().path().join("sync-dir");

    let imap_config = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: 3143,
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..ImapConfig::default()
    });

    let account_config = Arc::new(AccountConfig {
        name: "account".into(),
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([(SENT.into(), "[Gmail]/Sent".into())])),
            ..Default::default()
        }),
        sync: Some(SyncConfig {
            enable: Some(true),
            dir: Some(sync_dir.clone()),
            ..Default::default()
        }),
        ..Default::default()
    });

    // set up imap

    let imap_ctx = ImapContextBuilder::new(imap_config.clone());
    let imap_builder = BackendBuilder::new(account_config.clone(), imap_ctx);
    let imap = imap_builder.clone().build().await.unwrap();

    // set up maildir reader

    let mdir_ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig {
        root_dir: sync_dir.clone(),
    }));
    let mdir_builder = BackendBuilder::new(account_config.clone(), mdir_ctx);
    let mdir = mdir_builder.clone().build().await.unwrap();

    // set up folders

    for folder in imap.list_folders().await.unwrap().iter() {
        if folder.is_inbox() {
            imap.purge_folder(INBOX).await.unwrap()
        } else {
            imap.delete_folder(&folder.name).await.unwrap()
        }
    }

    imap.add_folder("[Gmail]/Sent").await.unwrap();
    imap.add_folder(TRASH).await.unwrap();

    // add three emails to folder INBOX with delay (in order to have
    // different dates)

    imap.add_message_with_flag(
        INBOX,
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

    imap.add_message_with_flags(
        INBOX,
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

    imap.add_message_with_flags(
        INBOX,
        &MessageBuilder::new()
            .message_id("<c@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("C")
            .text_body("C")
            .write_to_vec()
            .unwrap(),
        &Flags::default(),
    )
    .await
    .unwrap();

    let imap_inbox_envelopes = imap.list_envelopes(INBOX, 0, 0).await.unwrap();

    // add two more emails to folder [Gmail]/Sent

    imap.add_message_with_flags(
        "sent",
        &MessageBuilder::new()
            .message_id("<d@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("D")
            .text_body("D")
            .write_to_vec()
            .unwrap(),
        &Flags::default(),
    )
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    imap.add_message_with_flags(
        "SenT",
        &MessageBuilder::new()
            .message_id("<e@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("E")
            .text_body("E")
            .write_to_vec()
            .unwrap(),
        &Flags::default(),
    )
    .await
    .unwrap();

    let imap_sent_envelopes = imap.list_envelopes(SENT, 0, 0).await.unwrap();

    // sync imap account twice in a row to see if all work as expected
    // without duplicate items
    let sync_builder = AccountSyncBuilder::new(account_config.clone(), mdir_builder, imap_builder)
        .await
        .unwrap();
    sync_builder.sync().await.unwrap();
    sync_builder.sync().await.unwrap();

    // check folders integrity

    let mut imap_folders = imap.list_folders().await.unwrap();
    imap_folders.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(imap_folders.len(), 3);
    assert_eq!(imap_folders[0].name, "INBOX");
    assert_eq!(imap_folders[0].kind, Some(FolderKind::Inbox));
    assert_eq!(imap_folders[1].name, "Trash");
    assert_eq!(imap_folders[1].kind, Some(FolderKind::Trash));
    assert_eq!(imap_folders[2].name, "[Gmail]/Sent");
    assert_eq!(imap_folders[2].kind, Some(FolderKind::Sent));

    let mut mdir_folders = mdir.list_folders().await.unwrap();
    mdir_folders.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(imap_folders, mdir_folders);

    // check maildir envelopes integrity

    let mdir_inbox_envelopes = mdir.list_envelopes(INBOX, 0, 0).await.unwrap();
    assert_eq!(imap_inbox_envelopes, mdir_inbox_envelopes);

    let mdir_sent_envelopes = mdir.list_envelopes(SENT, 0, 0).await.unwrap();
    assert_eq!(imap_sent_envelopes, mdir_sent_envelopes);

    // check maildir emails content integrity

    let ids = Id::multiple(mdir_inbox_envelopes.iter().map(|e| &e.id));
    let msgs = mdir.get_messages(INBOX, &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(3, msgs.len());
    assert_eq!("C", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("B", msgs[1].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("A", msgs[2].parsed().unwrap().body_text(0).unwrap());

    let ids = Id::multiple(mdir_sent_envelopes.iter().map(|e| &e.id));
    let msgs = mdir.get_messages(SENT, &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(2, msgs.len());
    assert_eq!("E", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("D", msgs[1].parsed().unwrap().body_text(0).unwrap());

    // check folders cache integrity

    let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite")).unwrap();

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Include(HashSet::from_iter([SENT.into()])),
    )
    .unwrap();
    assert!(!local_folders_cached.contains(INBOX));
    assert!(local_folders_cached.contains(SENT));

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Exclude(HashSet::from_iter([SENT.into()])),
    )
    .unwrap();
    assert!(local_folders_cached.contains(INBOX));
    assert!(!local_folders_cached.contains(SENT));

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::All,
    )
    .unwrap();
    assert!(local_folders_cached.contains(INBOX));
    assert!(local_folders_cached.contains(SENT));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Include(HashSet::from_iter([SENT.into()])),
    )
    .unwrap();
    assert!(!remote_folders_cached.contains(INBOX));
    assert!(remote_folders_cached.contains(SENT));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Exclude(HashSet::from_iter([SENT.into()])),
    )
    .unwrap();
    assert!(remote_folders_cached.contains(INBOX));
    assert!(!remote_folders_cached.contains(SENT));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::All,
    )
    .unwrap();
    assert!(remote_folders_cached.contains(INBOX));
    assert!(remote_folders_cached.contains(SENT));

    // CHECK envelopes cache integrity

    let mdir_inbox_envelopes_cached =
        EmailSyncCache::list_local_envelopes(&mut conn, &account_config.name, INBOX).unwrap();
    let imap_inbox_envelopes_cached =
        EmailSyncCache::list_remote_envelopes(&mut conn, &account_config.name, INBOX).unwrap();

    assert_eq!(mdir_inbox_envelopes, mdir_inbox_envelopes_cached);
    assert_eq!(imap_inbox_envelopes, imap_inbox_envelopes_cached);

    let mdir_sent_envelopes_cached =
        EmailSyncCache::list_local_envelopes(&mut conn, &account_config.name, SENT).unwrap();
    let imap_sent_envelopes_cached =
        EmailSyncCache::list_remote_envelopes(&mut conn, &account_config.name, SENT).unwrap();

    assert_eq!(mdir_sent_envelopes, mdir_sent_envelopes_cached);
    assert_eq!(imap_sent_envelopes, imap_sent_envelopes_cached);

    // remove emails and update flags from both side, sync again and
    // check integrity

    imap.delete_messages(INBOX, &Id::single(&imap_inbox_envelopes[0].id))
        .await
        .unwrap();
    imap.add_flags(
        INBOX,
        &Id::single(&imap_inbox_envelopes[1].id),
        &Flags::from_iter([Flag::Draft]),
    )
    .await
    .unwrap();
    imap.expunge_folder(INBOX).await.unwrap();
    mdir.delete_messages(INBOX, &Id::single(&mdir_inbox_envelopes[2].id))
        .await
        .unwrap();
    mdir.add_flags(
        INBOX,
        &Id::single(&mdir_inbox_envelopes[1].id),
        &Flags::from_iter([Flag::Flagged, Flag::Answered]),
    )
    .await
    .unwrap();
    mdir.expunge_folder(INBOX).await.unwrap();

    let report = sync_builder.sync().await.unwrap();
    assert_eq!(
        report.folders,
        HashSet::from_iter([INBOX.into(), SENT.into(), TRASH.into()])
    );

    let imap_envelopes = imap.list_envelopes(INBOX, 0, 0).await.unwrap();
    let mdir_envelopes = mdir.list_envelopes(INBOX, 0, 0).await.unwrap();
    assert_eq!(imap_envelopes, mdir_envelopes);

    let cached_mdir_envelopes =
        EmailSyncCache::list_local_envelopes(&mut conn, &account_config.name, INBOX).unwrap();
    assert_eq!(cached_mdir_envelopes, mdir_envelopes);

    let cached_imap_envelopes =
        EmailSyncCache::list_remote_envelopes(&mut conn, &account_config.name, INBOX).unwrap();
    assert_eq!(cached_imap_envelopes, imap_envelopes);
}
