use email::{
    account::{
        config::{passwd::PasswdConfig, AccountConfig},
        sync::{AccountSyncBuilder, LocalBackendBuilder},
    },
    backend::BackendBuilder,
    email::sync::EmailSyncCache,
    envelope::{get::imap::GetEnvelopeImap, list::imap::ListEnvelopesImap, Id},
    flag::{add::imap::AddFlagsImap, set::imap::SetFlagsImap, Flag, Flags},
    folder::{
        self, add::imap::AddFolderImap, delete::imap::DeleteFolderImap,
        expunge::imap::ExpungeFolderImap, list::imap::ListFoldersImap,
        purge::imap::PurgeFolderImap,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapSessionBuilder,
    },
    maildir::config::MaildirConfig,
    message::{
        add_raw_with_flags::imap::AddRawMessageWithFlagsImap, get::imap::GetMessagesImap,
        move_::imap::MoveMessagesImap, peek::imap::PeekMessagesImap,
    },
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
    let account_config = AccountConfig {
        name: "account".into(),
        sync: true,
        sync_dir: Some(sync_dir.clone()),
        ..Default::default()
    };

    // set up imap

    let imap_ctx = ImapSessionBuilder::new(account_config.clone(), imap_config);
    let imap_builder = BackendBuilder::new(account_config.clone(), imap_ctx)
        .with_add_folder(AddFolderImap::new)
        .with_list_folders(ListFoldersImap::new)
        .with_expunge_folder(ExpungeFolderImap::new)
        .with_purge_folder(PurgeFolderImap::new)
        .with_delete_folder(DeleteFolderImap::new)
        .with_get_envelope(GetEnvelopeImap::new)
        .with_list_envelopes(ListEnvelopesImap::new)
        .with_add_flags(AddFlagsImap::new)
        .with_set_flags(SetFlagsImap::new)
        .with_peek_messages(PeekMessagesImap::new)
        .with_get_messages(GetMessagesImap::new)
        .with_move_messages(MoveMessagesImap::new)
        .with_add_raw_message_with_flags(AddRawMessageWithFlagsImap::new);
    let imap = imap_builder.clone().build().await.unwrap();

    // set up maildir reader

    let mdir = LocalBackendBuilder::new(
        account_config.clone(),
        MaildirConfig {
            root_dir: sync_dir.clone(),
        },
    )
    .build()
    .await
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

    imap.add_raw_message_with_flag(
        "INBOX",
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

    imap.add_raw_message_with_flags(
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

    imap.add_raw_message_with_flags(
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

    imap.add_raw_message_with_flags(
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

    imap.add_raw_message_with_flags(
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

    let sync_builder = AccountSyncBuilder::new(imap_builder).await.unwrap();
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

    let ids = Id::multiple(mdir_inbox_envelopes.iter().map(|e| &e.id));
    let msgs = mdir.get_messages("INBOX", &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(3, msgs.len());
    assert_eq!("C", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("B", msgs[1].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("A", msgs[2].parsed().unwrap().body_text(0).unwrap());

    let ids = Id::multiple(mdir_sent_envelopes.iter().map(|e| &e.id));
    let msgs = mdir.get_messages("[Gmail]/Sent", &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(2, msgs.len());
    assert_eq!("E", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("D", msgs[1].parsed().unwrap().body_text(0).unwrap());

    // check folders cache integrity

    let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite")).unwrap();

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(!local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::FolderSyncCache::list_local_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::All,
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(!remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::FolderSyncCache::list_remote_folders(
        &mut conn,
        &account_config.name,
        &folder::sync::FolderSyncStrategy::All,
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    // check envelopes cache integrity

    let mdir_inbox_envelopes_cached =
        EmailSyncCache::list_local_envelopes(&mut conn, &account_config.name, "INBOX").unwrap();
    let imap_inbox_envelopes_cached =
        EmailSyncCache::list_remote_envelopes(&mut conn, &account_config.name, "INBOX").unwrap();

    assert_eq!(mdir_inbox_envelopes, mdir_inbox_envelopes_cached);
    assert_eq!(imap_inbox_envelopes, imap_inbox_envelopes_cached);

    let mdir_sent_envelopes_cached =
        EmailSyncCache::list_local_envelopes(&mut conn, &account_config.name, "[Gmail]/Sent")
            .unwrap();
    let imap_sent_envelopes_cached =
        EmailSyncCache::list_remote_envelopes(&mut conn, &account_config.name, "[Gmail]/Sent")
            .unwrap();

    assert_eq!(mdir_sent_envelopes, mdir_sent_envelopes_cached);
    assert_eq!(imap_sent_envelopes, imap_sent_envelopes_cached);

    // remove emails and update flags from both side, sync again and
    // check integrity

    imap.delete_messages("INBOX", &Id::single(&imap_inbox_envelopes[0].id))
        .await
        .unwrap();
    imap.add_flags(
        "INBOX",
        &Id::single(&imap_inbox_envelopes[1].id),
        &Flags::from_iter([Flag::Draft]),
    )
    .await
    .unwrap();
    imap.expunge_folder("INBOX").await.unwrap();
    mdir.delete_messages("INBOX", &Id::single(&mdir_inbox_envelopes[2].id))
        .await
        .unwrap();
    mdir.add_flags(
        "INBOX",
        &Id::single(&mdir_inbox_envelopes[1].id),
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
        EmailSyncCache::list_local_envelopes(&mut conn, &account_config.name, "INBOX").unwrap();
    assert_eq!(cached_mdir_envelopes, mdir_envelopes);

    let cached_imap_envelopes =
        EmailSyncCache::list_remote_envelopes(&mut conn, &account_config.name, "INBOX").unwrap();
    assert_eq!(cached_imap_envelopes, imap_envelopes);
}
