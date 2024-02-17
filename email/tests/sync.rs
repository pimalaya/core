use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    email::sync::hunk::EmailSyncHunk,
    envelope::{Envelope, Id},
    flag::{Flag, Flags},
    folder::{
        config::FolderConfig,
        sync::{hunk::FolderSyncHunk, FolderSyncStrategy},
        Folder, FolderKind, INBOX, TRASH,
    },
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
use std::{
    collections::HashMap,
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::Arc,
};
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

    let right_ctx =
        ImapContextBuilder::new(account_config_right.clone(), imap_config_right.clone());
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

    // prepare sync builder

    static EVENTS_STACK: Lazy<Mutex<HashSet<SyncEvent>>> =
        Lazy::new(|| Mutex::const_new(HashSet::default()));

    let sync_builder = SyncBuilder::new(left_builder.clone(), right_builder.clone())
        .with_cache_dir(tmp.join("cache"))
        .with_handler(|evt| async {
            let mut stack = EVENTS_STACK.lock().await;
            stack.insert(evt);
            Ok(())
        });

    let left_cache = sync_builder
        .get_left_cache_builder()
        .unwrap()
        .build()
        .await
        .unwrap();
    let right_cache = sync_builder
        .get_right_cache_builder()
        .unwrap()
        .build()
        .await
        .unwrap();

    // check sync integrity with dry run on INBOX only

    let report = sync_builder
        .clone()
        .with_dry_run(true)
        .with_folders_filter(FolderSyncStrategy::Include(HashSet::from_iter([
            INBOX.into()
        ])))
        .sync()
        .await
        .unwrap();

    let expected_folders = HashSet::from_iter([INBOX.into()]);

    assert_eq!(report.folder.names, expected_folders);

    let expected_evts = HashSet::from_iter([
        SyncEvent::ListedLeftCachedFolders(1),
        SyncEvent::ListedRightCachedFolders(1),
        SyncEvent::ListedLeftFolders(1),
        SyncEvent::ListedRightFolders(1),
        SyncEvent::ListedAllFolders,
        SyncEvent::GeneratedFolderPatch(BTreeMap::from_iter([(
            INBOX.into(),
            BTreeSet::from_iter([]),
        )])),
        SyncEvent::ProcessedAllFolderHunks,
        SyncEvent::ListedLeftCachedEnvelopes(INBOX.into(), 0),
        SyncEvent::ListedRightCachedEnvelopes(INBOX.into(), 0),
        SyncEvent::ListedLeftEnvelopes(INBOX.into(), 0),
        SyncEvent::ListedRightEnvelopes(INBOX.into(), 3),
        SyncEvent::GeneratedEmailPatch(BTreeMap::from_iter([(
            INBOX.into(),
            BTreeSet::from_iter([
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
            ]),
        )])),
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
        SyncEvent::ProcessedAllEmailHunks,
        SyncEvent::ExpungedAllFolders,
    ]);

    {
        let mut evts = EVENTS_STACK.lock().await;
        assert_eq!(*evts, expected_evts);
        evts.clear()
    }

    // check full sync integrity

    let report = sync_builder.clone().sync().await.unwrap();

    let expected_folders = HashSet::from_iter([INBOX.into(), "Archives".into(), TRASH.into()]);

    assert_eq!(report.folder.names, expected_folders);

    let expected_evts = HashSet::from_iter([
        SyncEvent::ListedLeftCachedFolders(1),
        SyncEvent::ListedRightCachedFolders(1),
        SyncEvent::ListedLeftFolders(1),
        SyncEvent::ListedRightFolders(3),
        SyncEvent::ListedAllFolders,
        SyncEvent::GeneratedFolderPatch(BTreeMap::from_iter([
            (INBOX.into(), BTreeSet::from_iter([])),
            (
                "Archives".into(),
                BTreeSet::from_iter([
                    FolderSyncHunk::Cache("Archives".into(), SyncDestination::Right),
                    FolderSyncHunk::Create("Archives".into(), SyncDestination::Left),
                    FolderSyncHunk::Cache("Archives".into(), SyncDestination::Left),
                ]),
            ),
            (
                "Trash".into(),
                BTreeSet::from_iter([
                    FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Right),
                    FolderSyncHunk::Create(TRASH.into(), SyncDestination::Left),
                    FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Left),
                ]),
            ),
        ])),
        SyncEvent::ProcessedAllFolderHunks,
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
        SyncEvent::GeneratedEmailPatch(BTreeMap::from_iter([
            ("Archives".into(), BTreeSet::from_iter([])),
            (
                INBOX.into(),
                BTreeSet::from_iter([
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
                ]),
            ),
            (
                TRASH.into(),
                BTreeSet::from_iter([
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
                ]),
            ),
        ])),
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
        SyncEvent::ProcessedAllEmailHunks,
        SyncEvent::ExpungedAllFolders,
    ]);

    {
        let mut evts = EVENTS_STACK.lock().await;
        println!("diff: {:#?}", (*evts).difference(&expected_evts));
        assert_eq!(*evts, expected_evts);
        evts.clear()
    }

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

    // attempt a second sync that should lead to an empty report

    let report = sync_builder.clone().sync().await.unwrap();

    assert!(report.folder.patch.is_empty());
    assert!(report.email.patch.is_empty());

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

    let left_cached_folders: HashSet<Folder> =
        HashSet::from_iter::<Vec<Folder>>(left_cache.list_folders().await.unwrap().into());

    assert_eq!(left_cached_folders, left_folders);

    let right_cached_folders: HashSet<Folder> =
        HashSet::from_iter::<Vec<Folder>>(right_cache.list_folders().await.unwrap().into());

    assert_eq!(right_cached_folders, right_folders);

    // check envelopes integrity

    for folder in [INBOX, "Archives", TRASH] {
        let mut left_envelopes = left.list_envelopes(folder, 0, 0).await.unwrap();
        left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let mut left_cached_envelopes = left_cache.list_envelopes(folder, 0, 0).await.unwrap();
        left_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let mut right_envelopes = right.list_envelopes(folder, 0, 0).await.unwrap();
        right_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let mut right_cached_envelopes = right_cache.list_envelopes(folder, 0, 0).await.unwrap();
        right_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        assert_eq!(left_envelopes, left_cached_envelopes);
        assert_eq!(right_envelopes, right_cached_envelopes);
        assert_eq!(left_envelopes, right_envelopes);
    }

    // check left emails content integrity

    let mut left_envelopes = left.list_envelopes(INBOX, 0, 0).await.unwrap();
    left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let ids = Id::multiple(left_envelopes.iter().map(|e| &e.id));
    let msgs = left.peek_messages(INBOX, &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(3, msgs.len());
    assert_eq!("C", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("B", msgs[1].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("A", msgs[2].parsed().unwrap().body_text(0).unwrap());

    let mut left_envelopes = left.list_envelopes(TRASH, 0, 0).await.unwrap();
    left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let ids = Id::multiple(left_envelopes.iter().map(|e| &e.id));
    let msgs = left.peek_messages(TRASH, &ids).await.unwrap();
    let msgs = msgs.to_vec();
    assert_eq!(2, msgs.len());
    assert_eq!("E", msgs[0].parsed().unwrap().body_text(0).unwrap());
    assert_eq!("D", msgs[1].parsed().unwrap().body_text(0).unwrap());

    // remove messages and update flags from both side, sync again and
    // check integrity

    let mut left_envelopes = left.list_envelopes(INBOX, 0, 0).await.unwrap();
    left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    left.delete_messages(INBOX, &Id::single(&left_envelopes[2].id))
        .await
        .unwrap();
    left.add_flags(
        INBOX,
        &Id::single(&left_envelopes[1].id),
        &Flags::from_iter([Flag::Flagged, Flag::Answered]),
    )
    .await
    .unwrap();
    left.expunge_folder(INBOX).await.unwrap();

    let mut right_envelopes = right.list_envelopes(INBOX, 0, 0).await.unwrap();
    right_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    right
        .delete_messages(INBOX, &Id::single(&right_envelopes[0].id))
        .await
        .unwrap();
    right
        .add_flag(INBOX, &Id::single(&right_envelopes[1].id), Flag::Draft)
        .await
        .unwrap();
    right.expunge_folder(INBOX).await.unwrap();

    let report = sync_builder.sync().await.unwrap();

    let mut left_envelopes = left.list_envelopes(INBOX, 0, 0).await.unwrap();
    left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let mut left_cached_envelopes = left_cache.list_envelopes(INBOX, 0, 0).await.unwrap();
    left_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let mut right_envelopes = right.list_envelopes(INBOX, 0, 0).await.unwrap();
    right_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    let mut right_cached_envelopes = right_cache.list_envelopes(INBOX, 0, 0).await.unwrap();
    right_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

    assert!(report.folder.patch.is_empty());
    assert!(!report.email.patch.is_empty());
    assert_eq!(left_envelopes, left_cached_envelopes);
    assert_eq!(right_envelopes, right_cached_envelopes);
    assert_eq!(left_envelopes, right_envelopes);
}
