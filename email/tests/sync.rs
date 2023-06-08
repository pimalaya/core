use env_logger;
use mail_builder::MessageBuilder;
use pimalaya_secret::Secret;
use std::{borrow::Cow, collections::HashSet, thread, time::Duration};
use tempfile::tempdir;

use pimalaya_email::{
    envelope, folder, AccountConfig, Backend, BackendBuilder, BackendConfig, BackendSyncBuilder,
    Flag, Flags, ImapAuthConfig, ImapConfig, MaildirBackend, MaildirConfig, PasswdConfig,
};

#[test]
fn sync() {
    env_logger::builder().is_test(true).init();

    // set up config

    let sync_dir = tempdir().unwrap().path().join("sync-dir");
    let config = AccountConfig {
        name: "account".into(),
        sync: true,
        sync_dir: Some(sync_dir.clone()),
        backend: BackendConfig::Imap(ImapConfig {
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
        }),
        ..AccountConfig::default()
    };

    // set up imap

    let mut imap_builder = BackendBuilder::new(Cow::Borrowed(&config));
    let mut imap = imap_builder
        .clone()
        .with_cache_disabled(true)
        .build_into()
        .unwrap();

    // set up maildir reader

    let mut mdir = MaildirBackend::new(
        Cow::Borrowed(&config),
        Cow::Owned(MaildirConfig {
            root_dir: sync_dir.clone(),
        }),
    )
    .unwrap();

    // set up folders

    if let Err(_) = imap.delete_folder("[Gmail]/Sent") {}
    if let Err(_) = imap.delete_folder("Trash") {}
    imap.purge_folder("INBOX").unwrap();
    imap.add_folder("[Gmail]/Sent").unwrap();
    imap.add_folder("Trash").unwrap();

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
    .unwrap();

    thread::sleep(Duration::from_secs(1));

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
    .unwrap();

    thread::sleep(Duration::from_secs(1));

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
    .unwrap();

    let imap_inbox_envelopes = imap.list_envelopes("INBOX", 0, 0).unwrap();

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
    .unwrap();

    thread::sleep(Duration::from_secs(1));

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
    .unwrap();

    let imap_sent_envelopes = imap.list_envelopes("[Gmail]/Sent", 0, 0).unwrap();

    // sync imap account twice in a row to see if all work as expected
    // without duplicate items

    let sync_builder = BackendSyncBuilder::new(&config);
    sync_builder.sync(&mut imap_builder).unwrap();
    sync_builder.sync(&mut imap_builder).unwrap();

    // check folders integrity

    let imap_folders = imap
        .list_folders()
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
        .unwrap()
        .iter()
        .map(|f| f.name.clone())
        .collect::<HashSet<_>>();

    assert_eq!(imap_folders, mdir_folders);

    // check maildir envelopes integrity

    let mdir_inbox_envelopes = mdir.list_envelopes("INBOX", 0, 0).unwrap();
    assert_eq!(imap_inbox_envelopes, mdir_inbox_envelopes);

    let mdir_sent_envelopes = mdir.list_envelopes("[Gmail]/Sent", 0, 0).unwrap();
    assert_eq!(imap_sent_envelopes, mdir_sent_envelopes);

    // check maildir emails content integrity

    let ids = mdir_inbox_envelopes.iter().map(|e| e.id.as_str()).collect();
    let emails = mdir.get_emails("INBOX", ids).unwrap();
    let emails = emails.to_vec();
    assert_eq!(3, emails.len());
    assert_eq!("C", emails[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("B", emails[1].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("A", emails[2].parsed().unwrap().body_text(0).unwrap());

    let ids = mdir_sent_envelopes.iter().map(|e| e.id.as_str()).collect();
    let emails = mdir.get_emails("[Gmail]/Sent", ids).unwrap();
    let emails = emails.to_vec();
    assert_eq!(2, emails.len());
    assert_eq!("E", emails[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("D", emails[1].parsed().unwrap().body_text(0).unwrap());

    // check folders cache integrity

    let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite")).unwrap();

    let local_folders_cached = folder::sync::Cache::list_local_folders(
        &mut conn,
        &config.name,
        &folder::sync::Strategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::Cache::list_local_folders(
        &mut conn,
        &config.name,
        &folder::sync::Strategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(!local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::Cache::list_local_folders(
        &mut conn,
        &config.name,
        &folder::sync::Strategy::All,
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::Cache::list_remote_folders(
        &mut conn,
        &config.name,
        &folder::sync::Strategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::Cache::list_remote_folders(
        &mut conn,
        &config.name,
        &folder::sync::Strategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(!remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::Cache::list_remote_folders(
        &mut conn,
        &config.name,
        &folder::sync::Strategy::All,
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    // check envelopes cache integrity

    let mdir_inbox_envelopes_cached =
        envelope::sync::Cache::list_local_envelopes(&mut conn, &config.name, "INBOX").unwrap();
    let imap_inbox_envelopes_cached =
        envelope::sync::Cache::list_remote_envelopes(&mut conn, &config.name, "INBOX").unwrap();

    assert_eq!(mdir_inbox_envelopes, mdir_inbox_envelopes_cached);
    assert_eq!(imap_inbox_envelopes, imap_inbox_envelopes_cached);

    let mdir_sent_envelopes_cached =
        envelope::sync::Cache::list_local_envelopes(&mut conn, &config.name, "[Gmail]/Sent")
            .unwrap();
    let imap_sent_envelopes_cached =
        envelope::sync::Cache::list_remote_envelopes(&mut conn, &config.name, "[Gmail]/Sent")
            .unwrap();

    assert_eq!(mdir_sent_envelopes, mdir_sent_envelopes_cached);
    assert_eq!(imap_sent_envelopes, imap_sent_envelopes_cached);

    // remove emails and update flags from both side, sync again and
    // check integrity

    imap.delete_emails("INBOX", vec![&imap_inbox_envelopes[0].id])
        .unwrap();
    imap.add_flags(
        "INBOX",
        vec![&imap_inbox_envelopes[1].id],
        &Flags::from_iter([Flag::Draft]),
    )
    .unwrap();
    imap.expunge_folder("INBOX").unwrap();
    mdir.delete_emails("INBOX", vec![&mdir_inbox_envelopes[2].id])
        .unwrap();
    mdir.add_flags(
        "INBOX",
        vec![&mdir_inbox_envelopes[1].id],
        &Flags::from_iter([Flag::Flagged, Flag::Answered]),
    )
    .unwrap();
    mdir.expunge_folder("INBOX").unwrap();

    let report = sync_builder.sync(&mut imap_builder).unwrap();
    assert_eq!(
        report.folders,
        HashSet::from_iter(["INBOX".into(), "[Gmail]/Sent".into(), "Trash".into()])
    );

    let imap_envelopes = imap.list_envelopes("INBOX", 0, 0).unwrap();
    let mdir_envelopes = mdir.list_envelopes("INBOX", 0, 0).unwrap();
    assert_eq!(imap_envelopes, mdir_envelopes);

    let cached_mdir_envelopes =
        envelope::sync::Cache::list_local_envelopes(&mut conn, &config.name, "INBOX").unwrap();
    assert_eq!(cached_mdir_envelopes, mdir_envelopes);

    let cached_imap_envelopes =
        envelope::sync::Cache::list_remote_envelopes(&mut conn, &config.name, "INBOX").unwrap();
    assert_eq!(cached_imap_envelopes, imap_envelopes);
}
