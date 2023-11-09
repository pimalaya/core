use email::{
    account::{sync::AccountSyncBuilder, AccountConfig, PasswdConfig},
    backend::{
        Backend, BackendBuilder, BackendBuilderV2, BackendConfig, ImapAuthConfig, ImapConfig,
        MaildirBackend, MaildirConfig,
    },
    email::{sync::EmailSyncCache, Flag, Flags},
    folder::{
        self, add::imap::AddImapFolder, delete::imap::DeleteImapFolder, list::imap::ListImapFolders,
    },
    imap::ImapSessionBuilder,
};
use env_logger;
use mail_builder::MessageBuilder;
use secret::Secret;
use std::{collections::HashSet, time::Duration};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread")]
async fn sync() {
    env_logger::builder().is_test(true).init();

    // set up config

    let sync_dir = tempdir().unwrap().path().join("sync-dir");
    let imap_config = ImapConfig {
        host: "localhost".into(),
        port: 3143,
        ssl: Some(false),
        starttls: Some(false),
        insecure: Some(true),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig {
            passwd: Secret::new_raw("password"),
        }),
        ..ImapConfig::default()
    };
    let config = AccountConfig {
        name: "account".into(),
        sync: true,
        sync_dir: Some(sync_dir.clone()),
        backend: BackendConfig::Imap(imap_config.clone()),
        ..AccountConfig::default()
    };

    // set up imap

    let imap_builder = BackendBuilder::new(config.clone());
    let mut imap = imap_builder
        .clone()
        .with_cache_disabled(true)
        .into_build()
        .await
        .unwrap();

    // set up maildir reader

    let mut mdir = MaildirBackend::new(
        config.clone(),
        MaildirConfig {
            root_dir: sync_dir.clone(),
        },
    )
    .unwrap();

    // set up folders

    for folder in imap.list_folders().await.unwrap().iter() {
        match folder.name.as_str() {
            "INBOX" => imap.purge_folder("INBOX").await.unwrap(),
            folder => imap.delete_folder(folder).await.unwrap(),
        }
    }

    imap.add_folder("[Gmail]/Sent").await.unwrap();
    imap.add_folder("Trash").await.unwrap();

    // add three emails to folder INBOX with delay (in order to have
    // different dates)

    imap.add_email(
        "INBOX",
        &MessageBuilder::new()
            .message_id("<a@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("A")
            .text_body("A")
            .write_to_vec()
            .unwrap(),
        &Flags::from_iter([Flag::Seen]),
    )
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    imap.add_email(
        "INBOX",
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

    imap.add_email(
        "INBOX",
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

    let imap_inbox_envelopes = imap.list_envelopes("INBOX", 0, 0).await.unwrap();

    // add two more emails to folder [Gmail]/Sent

    imap.add_email(
        "[Gmail]/Sent",
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

    imap.add_email(
        "[Gmail]/Sent",
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

    let imap_sent_envelopes = imap.list_envelopes("[Gmail]/Sent", 0, 0).await.unwrap();

    // sync imap account twice in a row to see if all work as expected
    // without duplicate items

    let backend_context_v2 = ImapSessionBuilder::new(config.clone(), imap_config);
    let backend_builder_v2 = BackendBuilderV2::new(config.clone(), backend_context_v2)
        .with_add_folder(AddImapFolder::new)
        .with_list_folders(ListImapFolders::new)
        .with_delete_folder(DeleteImapFolder::new);

    let sync_builder = AccountSyncBuilder::new(config.clone(), imap_builder, backend_builder_v2)
        .await
        .unwrap();
    sync_builder.sync().await.unwrap();
    sync_builder.sync().await.unwrap();

    // check folders integrity

    let imap_folders = imap
        .list_folders()
        .await
        .unwrap()
        .iter()
        .map(|f| f.name.clone())
        .collect::<HashSet<_>>();

    assert_eq!(
        imap_folders,
        HashSet::from_iter([
            "INBOX".to_owned(),
            "Trash".to_owned(),
            "[Gmail]/Sent".to_owned()
        ])
    );

    let mdir_folders = mdir
        .list_folders()
        .await
        .unwrap()
        .iter()
        .map(|f| f.name.clone())
        .collect::<HashSet<_>>();

    assert_eq!(imap_folders, mdir_folders);

    // check maildir envelopes integrity

    let mdir_inbox_envelopes = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    assert_eq!(imap_inbox_envelopes, mdir_inbox_envelopes);

    let mdir_sent_envelopes = mdir.list_envelopes("[Gmail]/Sent", 0, 0).await.unwrap();
    assert_eq!(imap_sent_envelopes, mdir_sent_envelopes);

    // check maildir emails content integrity

    let ids = mdir_inbox_envelopes.iter().map(|e| e.id.as_str()).collect();
    let emails = mdir.get_emails("INBOX", ids).await.unwrap();
    let emails = emails.to_vec();
    assert_eq!(3, emails.len());
    assert_eq!("C", emails[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("B", emails[1].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("A", emails[2].parsed().unwrap().body_text(0).unwrap());

    let ids = mdir_sent_envelopes.iter().map(|e| e.id.as_str()).collect();
    let emails = mdir.get_emails("[Gmail]/Sent", ids).await.unwrap();
    let emails = emails.to_vec();
    assert_eq!(2, emails.len());
    assert_eq!("E", emails[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("D", emails[1].parsed().unwrap().body_text(0).unwrap());

    // check folders cache integrity

    let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite")).unwrap();

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &config.name,
        &folder::sync::FolderSyncStrategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &config.name,
        &folder::sync::FolderSyncStrategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(!local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &config.name,
        &folder::sync::FolderSyncStrategy::All,
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &config.name,
        &folder::sync::FolderSyncStrategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &config.name,
        &folder::sync::FolderSyncStrategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(!remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &config.name,
        &folder::sync::FolderSyncStrategy::All,
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    // check envelopes cache integrity

    let mdir_inbox_envelopes_cached =
        EmailSyncCache::list_local_envelopes(&mut conn, &config.name, "INBOX").unwrap();
    let imap_inbox_envelopes_cached =
        EmailSyncCache::list_remote_envelopes(&mut conn, &config.name, "INBOX").unwrap();

    assert_eq!(mdir_inbox_envelopes, mdir_inbox_envelopes_cached);
    assert_eq!(imap_inbox_envelopes, imap_inbox_envelopes_cached);

    let mdir_sent_envelopes_cached =
        EmailSyncCache::list_local_envelopes(&mut conn, &config.name, "[Gmail]/Sent").unwrap();
    let imap_sent_envelopes_cached =
        EmailSyncCache::list_remote_envelopes(&mut conn, &config.name, "[Gmail]/Sent").unwrap();

    assert_eq!(mdir_sent_envelopes, mdir_sent_envelopes_cached);
    assert_eq!(imap_sent_envelopes, imap_sent_envelopes_cached);

    // remove emails and update flags from both side, sync again and
    // check integrity

    imap.delete_emails("INBOX", vec![&imap_inbox_envelopes[0].id])
        .await
        .unwrap();
    imap.add_flags(
        "INBOX",
        vec![&imap_inbox_envelopes[1].id],
        &Flags::from_iter([Flag::Draft]),
    )
    .await
    .unwrap();
    imap.expunge_folder("INBOX").await.unwrap();
    mdir.delete_emails("INBOX", vec![&mdir_inbox_envelopes[2].id])
        .await
        .unwrap();
    mdir.add_flags(
        "INBOX",
        vec![&mdir_inbox_envelopes[1].id],
        &Flags::from_iter([Flag::Flagged, Flag::Answered]),
    )
    .await
    .unwrap();
    mdir.expunge_folder("INBOX").await.unwrap();

    let report = sync_builder.sync().await.unwrap();
    assert_eq!(
        report.folders,
        HashSet::from_iter(["INBOX".into(), "[Gmail]/Sent".into(), "Trash".into()])
    );

    let imap_envelopes = imap.list_envelopes("INBOX", 0, 0).await.unwrap();
    let mdir_envelopes = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    assert_eq!(imap_envelopes, mdir_envelopes);

    let cached_mdir_envelopes =
        EmailSyncCache::list_local_envelopes(&mut conn, &config.name, "INBOX").unwrap();
    assert_eq!(cached_mdir_envelopes, mdir_envelopes);

    let cached_imap_envelopes =
        EmailSyncCache::list_remote_envelopes(&mut conn, &config.name, "INBOX").unwrap();
    assert_eq!(cached_imap_envelopes, imap_envelopes);
}
