use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    email::sync::EmailSyncHunk,
    envelope::{Envelope, Id},
    flag::{Flag, Flags},
    folder::{config::FolderConfig, sync::FolderSyncHunk, Folder, FolderKind, INBOX, TRASH},
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    sync::{SyncBuilder, SyncDestination, SyncEvent},
};
use env_logger;
use mail_builder::MessageBuilder;
use once_cell::sync::Lazy;
use secret::Secret;
use std::{collections::HashMap, collections::HashSet, sync::Arc};
use tempfile::tempdir;
use tokio::sync::Mutex;

#[tokio::test]
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
            aliases: Some(HashMap::from_iter([(TRASH.into(), "[Gmail]/Trash".into())])),
            ..Default::default()
        }),
        ..Default::default()
    });

    // set up left backend (Maildir)

    let left_ctx = MaildirContextBuilder::new(mdir_config_left);
    let left_builder = BackendBuilder::new(account_config_left.clone(), left_ctx);
    let left = left_builder.clone().build().await.unwrap();

    // set up right backend (IMAP)

    let right_ctx = ImapContextBuilder::new(imap_config_right.clone());
    let right_builder = BackendBuilder::new(account_config_right.clone(), right_ctx);
    let right = right_builder.clone().build().await.unwrap();

    // reset right backend folders (keep only INBOX, Archives and [Gmail]/Trash)

    for folder in right.list_folders().await.unwrap().iter() {
        let _ = right.purge_folder(&folder.name).await;
        let _ = right.delete_folder(&folder.name).await;
    }

    right.add_folder("Archives").await.unwrap();
    right.add_folder("[Gmail]/Trash").await.unwrap();

    // add three messages to right INBOX folder

    right
        .add_message_with_flag(
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

    right
        .add_message_with_flags(
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

    right
        .add_message(
            INBOX,
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

    // add two more emails to right folder [Gmail]/Trash

    right
        .add_message_with_flags(
            "[Gmail]/Trash",
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

    right
        .add_message_with_flags(
            "TrAsH",
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

    // prepare sync builder then sync once

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

    // check sync report integrity

    let expected_folders = HashSet::from_iter([INBOX.into(), "Archives".into(), TRASH.into()]);

    assert_eq!(report.folder.folders, expected_folders);

    let evts = EVENTS_STACK.lock().await;
    let expected_evts = HashSet::from_iter([
        SyncEvent::ListedLeftCachedFolders(1),
        SyncEvent::ListedRightCachedFolders(1),
        SyncEvent::ListedLeftFolders(1),
        SyncEvent::ListedRightFolders(3),
        SyncEvent::ListedAllFolders,
        SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
            "Archives".into(),
            SyncDestination::Right,
        )),
        SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(
            "Archives".into(),
            SyncDestination::Left,
        )),
        SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
            "Archives".into(),
            SyncDestination::Left,
        )),
        SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Right)),
        SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(TRASH.into(), SyncDestination::Left)),
        SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Left)),
        SyncEvent::ListedLeftCachedEnvelopes(INBOX.into(), 0),
        SyncEvent::ListedRightCachedEnvelopes(INBOX.into(), 0),
        SyncEvent::ListedLeftEnvelopes(INBOX.into(), 0),
        SyncEvent::ListedRightEnvelopes(INBOX.into(), 3),
        SyncEvent::ListedLeftCachedEnvelopes("Archives".into(), 0),
        SyncEvent::ListedRightCachedEnvelopes("Archives".into(), 0),
        SyncEvent::ListedLeftEnvelopes("Archives".into(), 0),
        SyncEvent::ListedRightEnvelopes("Archives".into(), 0),
        SyncEvent::ListedLeftCachedEnvelopes(TRASH.into(), 0),
        SyncEvent::ListedRightCachedEnvelopes(TRASH.into(), 0),
        SyncEvent::ListedLeftEnvelopes(TRASH.into(), 0),
        SyncEvent::ListedRightEnvelopes(TRASH.into(), 2),
        SyncEvent::ListedAllEnvelopes,
        SyncEvent::ProcessedEmailHunk(EmailSyncHunk::CopyThenCache(
            INBOX.into(),
            Envelope {
                message_id: "<a@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )),
        SyncEvent::ProcessedEmailHunk(EmailSyncHunk::CopyThenCache(
            INBOX.into(),
            Envelope {
                message_id: "<b@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )),
        SyncEvent::ProcessedEmailHunk(EmailSyncHunk::CopyThenCache(
            INBOX.into(),
            Envelope {
                message_id: "<c@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )),
        SyncEvent::ProcessedEmailHunk(EmailSyncHunk::CopyThenCache(
            TRASH.into(),
            Envelope {
                message_id: "<d@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )),
        SyncEvent::ProcessedEmailHunk(EmailSyncHunk::CopyThenCache(
            TRASH.into(),
            Envelope {
                message_id: "<e@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )),
    ]);

    assert_eq!(*evts, expected_evts);

    let folder_patch: HashSet<_> = report
        .folder
        .patch
        .into_iter()
        .map(|(hunk, _err)| hunk)
        .collect();

    let expected_folder_patch = HashSet::from_iter([
        FolderSyncHunk::Cache("Archives".into(), SyncDestination::Right),
        FolderSyncHunk::Create("Archives".into(), SyncDestination::Left),
        FolderSyncHunk::Cache("Archives".into(), SyncDestination::Left),
        FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Right),
        FolderSyncHunk::Create(TRASH.into(), SyncDestination::Left),
        FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Left),
    ]);

    assert_eq!(folder_patch, expected_folder_patch);

    let email_patch: HashSet<_> = report
        .email
        .patch
        .into_iter()
        .map(|(hunk, _err)| hunk)
        .collect();

    let expected_email_patch = HashSet::from_iter([
        EmailSyncHunk::CopyThenCache(
            INBOX.into(),
            Envelope {
                message_id: "<a@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        ),
        EmailSyncHunk::CopyThenCache(
            INBOX.into(),
            Envelope {
                message_id: "<b@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        ),
        EmailSyncHunk::CopyThenCache(
            INBOX.into(),
            Envelope {
                message_id: "<c@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        ),
        EmailSyncHunk::CopyThenCache(
            TRASH.into(),
            Envelope {
                message_id: "<d@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        ),
        EmailSyncHunk::CopyThenCache(
            TRASH.into(),
            Envelope {
                message_id: "<e@localhost>".into(),
                ..Default::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        ),
    ]);

    assert_eq!(email_patch, expected_email_patch);

    // check folders integrity

    let right_folders: HashSet<Folder> =
        HashSet::from_iter::<Vec<Folder>>(right.list_folders().await.unwrap().into());
    let expected_right_folders = HashSet::from_iter([
        Folder {
            kind: Some(FolderKind::Inbox),
            name: INBOX.into(),
            ..Default::default()
        },
        Folder {
            kind: None,
            name: "Archives".into(),
            ..Default::default()
        },
        Folder {
            kind: Some(FolderKind::Trash),
            name: TRASH.into(),
            ..Default::default()
        },
    ]);

    assert_eq!(right_folders, expected_right_folders);

    let left_folders: HashSet<Folder> =
        HashSet::from_iter::<Vec<Folder>>(left.list_folders().await.unwrap().into());

    assert_eq!(left_folders, right_folders);

    // check left envelopes integrity

    let mut left_inbox_envelopes = left.list_envelopes(INBOX, 0, 0).await.unwrap();
    left_inbox_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let mut right_inbox_envelopes = right.list_envelopes(INBOX, 0, 0).await.unwrap();
    right_inbox_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    assert_eq!(left_inbox_envelopes, right_inbox_envelopes);

    let mut left_archives_envelopes = left.list_envelopes("Archives", 0, 0).await.unwrap();
    left_archives_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let mut right_archives_envelopes = right.list_envelopes("Archives", 0, 0).await.unwrap();
    right_archives_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    assert_eq!(left_archives_envelopes, right_archives_envelopes);

    let mut left_trash_envelopes = left.list_envelopes(TRASH, 0, 0).await.unwrap();
    left_trash_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let mut right_trash_envelopes = right.list_envelopes(TRASH, 0, 0).await.unwrap();
    right_trash_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    assert_eq!(left_trash_envelopes, right_trash_envelopes);

    // check left emails content integrity

    let ids = Id::multiple(left_inbox_envelopes.iter().map(|e| &e.id));
    let msgs = left.get_messages(INBOX, &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(3, msgs.len());
    assert_eq!("C", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("B", msgs[1].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("A", msgs[2].parsed().unwrap().body_text(0).unwrap());

    let ids = Id::multiple(left_trash_envelopes.iter().map(|e| &e.id));
    let msgs = left.get_messages(TRASH, &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(2, msgs.len());
    assert_eq!("E", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("D", msgs[1].parsed().unwrap().body_text(0).unwrap());

    // check folders cache integrity

    // TODO: generate left cache and right cache backends, then
    // continue test suite from account_sync.rs.
}
