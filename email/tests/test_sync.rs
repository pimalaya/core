use env_logger;
use pimalaya_secret::Secret;
use std::{borrow::Cow, collections::HashSet, thread, time::Duration};
use tempfile::tempdir;

use pimalaya_email::{
    envelope, folder, AccountConfig, Backend, BackendSyncBuilder, CompilerBuilder, Flag, Flags,
    ImapAuthConfig, ImapBackend, ImapConfig, MaildirBackend, MaildirConfig, PasswdConfig,
    TplBuilder,
};

#[test]
fn test_sync() {
    env_logger::builder().is_test(true).init();

    // set up account

    let sync_dir = tempdir().unwrap().path().join("sync-dir");

    let account = AccountConfig {
        name: "account".into(),
        sync: true,
        sync_dir: Some(sync_dir.clone()),
        ..AccountConfig::default()
    };

    // set up imap backend

    let imap = ImapBackend::new(
        Cow::Borrowed(&account),
        Cow::Owned(ImapConfig {
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
    )
    .unwrap();

    // set up folders

    imap.add_folder("[Gmail]/Sent").unwrap();
    imap.add_folder("Trash").unwrap();

    // add three emails to folder INBOX with delay (in order to have a
    // different date)

    imap.add_email(
        "INBOX",
        &TplBuilder::default()
            .message_id("<a@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("A")
            .text_plain_part("A")
            .compile(CompilerBuilder::default())
            .unwrap(),
        &Flags::from_iter([Flag::Seen]),
    )
    .unwrap();

    thread::sleep(Duration::from_secs(1));

    imap.add_email(
        "INBOX",
        &TplBuilder::default()
            .message_id("<b@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("B")
            .text_plain_part("B")
            .compile(CompilerBuilder::default())
            .unwrap(),
        &Flags::from_iter([Flag::Seen, Flag::Flagged, Flag::Custom("custom".into())]),
    )
    .unwrap();

    thread::sleep(Duration::from_secs(1));

    imap.add_email(
        "INBOX",
        &TplBuilder::default()
            .message_id("<c@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("C")
            .text_plain_part("C")
            .compile(CompilerBuilder::default())
            .unwrap(),
        &Flags::default(),
    )
    .unwrap();

    let imap_inbox_envelopes = imap.list_envelopes("INBOX", 0, 0).unwrap();

    // add two more emails to folder [Gmail]/Sent

    imap.add_email(
        "[Gmail]/Sent",
        &TplBuilder::default()
            .message_id("<d@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("D")
            .text_plain_part("D")
            .compile(CompilerBuilder::default())
            .unwrap(),
        &Flags::default(),
    )
    .unwrap();

    thread::sleep(Duration::from_secs(1));

    imap.add_email(
        "[Gmail]/Sent",
        &TplBuilder::default()
            .message_id("<e@localhost>")
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("E")
            .text_plain_part("E")
            .compile(CompilerBuilder::default())
            .unwrap(),
        &Flags::default(),
    )
    .unwrap();

    let imap_sent_envelopes = imap.list_envelopes("[Gmail]/Sent", 0, 0).unwrap();

    // set up maildir reader

    let mdir = MaildirBackend::new(
        Cow::Borrowed(&account),
        Cow::Owned(MaildirConfig {
            root_dir: sync_dir.clone(),
        }),
    )
    .unwrap();

    // sync imap account twice in a row to see if all work as expected
    // without duplicate items

    let sync_builder = BackendSyncBuilder::new(&account);
    sync_builder.sync(&imap).unwrap();
    // sync_builder.sync(&imap).unwrap();

    // check folders integrity

    let imap_folders = imap
        .list_folders()
        .unwrap()
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
    let mdir_folders = mdir
        .list_folders()
        .unwrap()
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();

    assert!(imap_folders.contains(&String::from("INBOX")));
    assert!(imap_folders.contains(&String::from("[Gmail]/Sent")));
    assert!(mdir_folders.contains(&String::from("INBOX")));
    assert!(mdir_folders.contains(&String::from("[Gmail]/Sent")));

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
    assert_eq!("C\r\n", emails[0].parsed().unwrap().get_body().unwrap());
    assert_eq!("B\r\n", emails[1].parsed().unwrap().get_body().unwrap());
    assert_eq!("A\r\n", emails[2].parsed().unwrap().get_body().unwrap());

    let ids = mdir_sent_envelopes.iter().map(|e| e.id.as_str()).collect();
    let emails = mdir.get_emails("[Gmail]/Sent", ids).unwrap();
    let emails = emails.to_vec();
    assert_eq!(2, emails.len());
    assert_eq!("E\r\n", emails[0].parsed().unwrap().get_body().unwrap());
    assert_eq!("D\r\n", emails[1].parsed().unwrap().get_body().unwrap());

    // check folders cache integrity

    let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite")).unwrap();

    let local_folders_cached = folder::sync::Cache::list_local_folders(
        &mut conn,
        &account.name,
        &folder::sync::Strategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::Cache::list_local_folders(
        &mut conn,
        &account.name,
        &folder::sync::Strategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(!local_folders_cached.contains("[Gmail]/Sent"));

    let local_folders_cached = folder::sync::Cache::list_local_folders(
        &mut conn,
        &account.name,
        &folder::sync::Strategy::All,
    )
    .unwrap();
    assert!(local_folders_cached.contains("INBOX"));
    assert!(local_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::Cache::list_remote_folders(
        &mut conn,
        &account.name,
        &folder::sync::Strategy::Include(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(!remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::Cache::list_remote_folders(
        &mut conn,
        &account.name,
        &folder::sync::Strategy::Exclude(HashSet::from_iter(["[Gmail]/Sent".into()])),
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(!remote_folders_cached.contains("[Gmail]/Sent"));

    let remote_folders_cached = folder::sync::Cache::list_remote_folders(
        &mut conn,
        &account.name,
        &folder::sync::Strategy::All,
    )
    .unwrap();
    assert!(remote_folders_cached.contains("INBOX"));
    assert!(remote_folders_cached.contains("[Gmail]/Sent"));

    // check envelopes cache integrity

    let mdir_inbox_envelopes_cached =
        envelope::sync::Cache::list_local_envelopes(&mut conn, &account.name, "INBOX").unwrap();
    let imap_inbox_envelopes_cached =
        envelope::sync::Cache::list_remote_envelopes(&mut conn, &account.name, "INBOX").unwrap();

    assert_eq!(mdir_inbox_envelopes, mdir_inbox_envelopes_cached);
    assert_eq!(imap_inbox_envelopes, imap_inbox_envelopes_cached);

    let mdir_sent_envelopes_cached =
        envelope::sync::Cache::list_local_envelopes(&mut conn, &account.name, "[Gmail]/Sent")
            .unwrap();
    let imap_sent_envelopes_cached =
        envelope::sync::Cache::list_remote_envelopes(&mut conn, &account.name, "[Gmail]/Sent")
            .unwrap();

    assert_eq!(mdir_sent_envelopes, mdir_sent_envelopes_cached);
    assert_eq!(imap_sent_envelopes, imap_sent_envelopes_cached);

    // remove emails and update flags from both side, sync again and
    // check integrity

    imap.delete_emails_internal("INBOX", vec![&imap_inbox_envelopes[0].internal_id])
        .unwrap();
    imap.add_flags_internal(
        "INBOX",
        vec![&imap_inbox_envelopes[1].internal_id],
        &Flags::from_iter([Flag::Draft]),
    )
    .unwrap();
    imap.expunge_folder("INBOX").unwrap();
    mdir.delete_emails_internal("INBOX", vec![&mdir_inbox_envelopes[2].internal_id])
        .unwrap();
    mdir.add_flags_internal(
        "INBOX",
        vec![&mdir_inbox_envelopes[1].internal_id],
        &Flags::from_iter([Flag::Flagged, Flag::Answered]),
    )
    .unwrap();
    mdir.expunge_folder("INBOX").unwrap();

    let report = sync_builder.sync(&imap).unwrap();
    assert_eq!(
        report.folders,
        HashSet::from_iter(["INBOX".into(), "[Gmail]/Sent".into(), "Trash".into()])
    );

    let imap_envelopes = imap.list_envelopes("INBOX", 0, 0).unwrap();
    let mdir_envelopes = mdir.list_envelopes("INBOX", 0, 0).unwrap();
    assert_eq!(imap_envelopes, mdir_envelopes);

    let cached_mdir_envelopes =
        envelope::sync::Cache::list_local_envelopes(&mut conn, &account.name, "INBOX").unwrap();
    assert_eq!(cached_mdir_envelopes, mdir_envelopes);

    let cached_imap_envelopes =
        envelope::sync::Cache::list_remote_envelopes(&mut conn, &account.name, "INBOX").unwrap();
    assert_eq!(cached_imap_envelopes, imap_envelopes);

    // clean up

    imap.purge_folder("INBOX").unwrap();
    imap.delete_folder("[Gmail]/Sent").unwrap();
    imap.delete_folder("Trash").unwrap();
    imap.close().unwrap();
}
