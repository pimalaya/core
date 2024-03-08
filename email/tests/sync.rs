use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::{Backend, BackendBuilder},
    email::sync::hunk::EmailSyncHunk,
    envelope::{list::ListEnvelopes, Envelope, Id},
    flag::{add::AddFlags, Flag, Flags},
    folder::{
        config::FolderConfig,
        expunge::ExpungeFolder,
        list::ListFolders,
        sync::{config::FolderSyncStrategy, hunk::FolderSyncHunk},
        Folder, FolderKind, DRAFTS, INBOX, SENT, TRASH,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder, ImapContextSync,
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder, MaildirContextSync},
    message::{add::AddMessage, delete::DeleteMessages, peek::PeekMessages},
    sync::{SyncBuilder, SyncDestination, SyncEvent},
};
use email_testing_server::with_email_testing_server;
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

    with_email_testing_server(|ports| async move {
        let tmp = tempdir().unwrap().path().to_owned();

        // set up left configs

        let mdir_config_left = Arc::new(MaildirConfig {
            root_dir: tmp.join("left"),
        });

        let account_config_left = Arc::new(AccountConfig {
            name: "left".into(),
            folder: Some(FolderConfig {
                aliases: Some(HashMap::from_iter([
                    (INBOX.into(), "INBOX".into()),
                    (SENT.into(), "Sent Items".into()),
                    (DRAFTS.into(), "Drafts".into()),
                    (TRASH.into(), "Deleted Items".into()),
                    ("Junk".into(), "Junk Mail".into()),
                ])),
                ..Default::default()
            }),
            ..Default::default()
        });

        // set up right configs

        let imap_config_right = Arc::new(ImapConfig {
            host: "localhost".into(),
            port: ports.imap,
            encryption: Some(ImapEncryptionKind::None),
            login: "bob".into(),
            auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
            ..ImapConfig::default()
        });

        let account_config_right = Arc::new(AccountConfig {
            name: "right".into(),
            folder: Some(FolderConfig {
                aliases: Some(HashMap::from_iter([
                    (INBOX.into(), "INBOX".into()),
                    (SENT.into(), "Sent Items".into()),
                    (DRAFTS.into(), "Drafts".into()),
                    (TRASH.into(), "Deleted Items".into()),
                    ("Junk".into(), "Junk Mail".into()),
                ])),
                ..Default::default()
            }),
            ..Default::default()
        });

        // set up left backend (Maildir)

        let left_ctx = MaildirContextBuilder::new(account_config_left.clone(), mdir_config_left);
        let left_builder = BackendBuilder::new(account_config_left.clone(), left_ctx);
        let left = left_builder
            .clone()
            .build::<Backend<MaildirContextSync>>()
            .await
            .unwrap();

        // set up right backend (IMAP)

        let right_ctx =
            ImapContextBuilder::new(account_config_right.clone(), imap_config_right.clone());
        let right_builder = BackendBuilder::new(account_config_right.clone(), right_ctx);
        let right = right_builder
            .clone()
            .build::<Backend<ImapContextSync>>()
            .await
            .unwrap();

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

        // add two more emails to right folder Junk Mail

        right
            .add_message_with_flags(
                "junk",
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
                "Junk Mail",
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
            .with_pool_size(1)
            .with_handler(|evt| async {
                let mut stack = EVENTS_STACK.lock().await;
                stack.insert(evt);
                Ok(())
            });

        let left_cache = sync_builder
            .get_left_cache_builder()
            .unwrap()
            .build::<Backend<MaildirContextSync>>()
            .await
            .unwrap();
        let right_cache = sync_builder
            .get_right_cache_builder()
            .unwrap()
            .build::<Backend<MaildirContextSync>>()
            .await
            .unwrap();

        // check sync integrity with dry run on INBOX only

        let report = sync_builder
            .clone()
            .with_dry_run(true)
            .with_folders_filter(FolderSyncStrategy::Include(BTreeSet::from_iter([
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

        let expected_folders = HashSet::from_iter([
            INBOX.into(),
            DRAFTS.into(),
            SENT.into(),
            TRASH.into(),
            "Junk".into(),
        ]);

        assert_eq!(report.folder.names, expected_folders);

        let expected_evts = HashSet::from_iter([
            SyncEvent::ListedLeftCachedFolders(1),
            SyncEvent::ListedRightCachedFolders(1),
            SyncEvent::ListedLeftFolders(1),
            SyncEvent::ListedRightFolders(5),
            SyncEvent::ListedAllFolders,
            SyncEvent::GeneratedFolderPatch(BTreeMap::from_iter([
                (INBOX.into(), BTreeSet::from_iter([])),
                (
                    SENT.into(),
                    BTreeSet::from_iter([
                        FolderSyncHunk::Cache(SENT.into(), SyncDestination::Right),
                        FolderSyncHunk::Create(SENT.into(), SyncDestination::Left),
                        FolderSyncHunk::Cache(SENT.into(), SyncDestination::Left),
                    ]),
                ),
                (
                    DRAFTS.into(),
                    BTreeSet::from_iter([
                        FolderSyncHunk::Cache(DRAFTS.into(), SyncDestination::Right),
                        FolderSyncHunk::Create(DRAFTS.into(), SyncDestination::Left),
                        FolderSyncHunk::Cache(DRAFTS.into(), SyncDestination::Left),
                    ]),
                ),
                (
                    TRASH.into(),
                    BTreeSet::from_iter([
                        FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Right),
                        FolderSyncHunk::Create(TRASH.into(), SyncDestination::Left),
                        FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Left),
                    ]),
                ),
                (
                    "Junk".into(),
                    BTreeSet::from_iter([
                        FolderSyncHunk::Cache("Junk".into(), SyncDestination::Right),
                        FolderSyncHunk::Create("Junk".into(), SyncDestination::Left),
                        FolderSyncHunk::Cache("Junk".into(), SyncDestination::Left),
                    ]),
                ),
            ])),
            SyncEvent::ProcessedAllFolderHunks,
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                SENT.into(),
                SyncDestination::Right,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(
                SENT.into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                SENT.into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                DRAFTS.into(),
                SyncDestination::Right,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(
                DRAFTS.into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                DRAFTS.into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                TRASH.into(),
                SyncDestination::Right,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(
                TRASH.into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                TRASH.into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                "Junk".into(),
                SyncDestination::Right,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Create(
                "Junk".into(),
                SyncDestination::Left,
            )),
            SyncEvent::ProcessedFolderHunk(FolderSyncHunk::Cache(
                "Junk".into(),
                SyncDestination::Left,
            )),
            SyncEvent::ListedLeftCachedEnvelopes(INBOX.into(), 0),
            SyncEvent::ListedRightCachedEnvelopes(INBOX.into(), 0),
            SyncEvent::ListedLeftEnvelopes(INBOX.into(), 0),
            SyncEvent::ListedRightEnvelopes(INBOX.into(), 3),
            SyncEvent::ListedLeftCachedEnvelopes(DRAFTS.into(), 0),
            SyncEvent::ListedRightCachedEnvelopes(DRAFTS.into(), 0),
            SyncEvent::ListedLeftEnvelopes(DRAFTS.into(), 0),
            SyncEvent::ListedRightEnvelopes(DRAFTS.into(), 0),
            SyncEvent::ListedLeftCachedEnvelopes(SENT.into(), 0),
            SyncEvent::ListedRightCachedEnvelopes(SENT.into(), 0),
            SyncEvent::ListedLeftEnvelopes(SENT.into(), 0),
            SyncEvent::ListedRightEnvelopes(SENT.into(), 0),
            SyncEvent::ListedLeftCachedEnvelopes(TRASH.into(), 0),
            SyncEvent::ListedRightCachedEnvelopes(TRASH.into(), 0),
            SyncEvent::ListedLeftEnvelopes(TRASH.into(), 0),
            SyncEvent::ListedRightEnvelopes(TRASH.into(), 0),
            SyncEvent::ListedLeftCachedEnvelopes("Junk".into(), 0),
            SyncEvent::ListedRightCachedEnvelopes("Junk".into(), 0),
            SyncEvent::ListedLeftEnvelopes("Junk".into(), 0),
            SyncEvent::ListedRightEnvelopes("Junk".into(), 2),
            SyncEvent::GeneratedEmailPatch(BTreeMap::from_iter([
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
                (DRAFTS.into(), BTreeSet::from_iter([])),
                (SENT.into(), BTreeSet::from_iter([])),
                (
                    "Junk".into(),
                    BTreeSet::from_iter([
                        EmailSyncHunk::CopyThenCache(
                            "Junk".into(),
                            Envelope {
                                message_id: "<d@localhost>".into(),
                                ..Default::default()
                            },
                            SyncDestination::Right,
                            SyncDestination::Left,
                            true,
                        ),
                        EmailSyncHunk::CopyThenCache(
                            "Junk".into(),
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
                (TRASH.into(), BTreeSet::from_iter([])),
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
                "Junk".into(),
                Envelope {
                    message_id: "<d@localhost>".into(),
                    ..Default::default()
                },
                SyncDestination::Right,
                SyncDestination::Left,
                true,
            )),
            SyncEvent::ProcessedEmailHunk(EmailSyncHunk::CopyThenCache(
                "Junk".into(),
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
            FolderSyncHunk::Cache(DRAFTS.into(), SyncDestination::Right),
            FolderSyncHunk::Create(DRAFTS.into(), SyncDestination::Left),
            FolderSyncHunk::Cache(DRAFTS.into(), SyncDestination::Left),
            FolderSyncHunk::Cache(SENT.into(), SyncDestination::Right),
            FolderSyncHunk::Create(SENT.into(), SyncDestination::Left),
            FolderSyncHunk::Cache(SENT.into(), SyncDestination::Left),
            FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Right),
            FolderSyncHunk::Create(TRASH.into(), SyncDestination::Left),
            FolderSyncHunk::Cache(TRASH.into(), SyncDestination::Left),
            FolderSyncHunk::Cache("Junk".into(), SyncDestination::Right),
            FolderSyncHunk::Create("Junk".into(), SyncDestination::Left),
            FolderSyncHunk::Cache("Junk".into(), SyncDestination::Left),
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
                "Junk".into(),
                Envelope {
                    message_id: "<d@localhost>".into(),
                    ..Default::default()
                },
                SyncDestination::Right,
                SyncDestination::Left,
                true,
            ),
            EmailSyncHunk::CopyThenCache(
                "Junk".into(),
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
                kind: Some(FolderKind::Drafts),
                name: DRAFTS.into(),
                ..Default::default()
            },
            Folder {
                kind: Some(FolderKind::Sent),
                name: SENT.into(),
                ..Default::default()
            },
            Folder {
                kind: Some(FolderKind::Trash),
                name: TRASH.into(),
                ..Default::default()
            },
            Folder {
                kind: Some(FolderKind::UserDefined("Junk".into())),
                name: "Junk Mail".into(),
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

        for folder in [INBOX, "Junk", TRASH] {
            let mut left_envelopes = left
                .list_envelopes(folder, Default::default())
                .await
                .unwrap();
            left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

            let mut left_cached_envelopes = left_cache
                .list_envelopes(folder, Default::default())
                .await
                .unwrap();
            left_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

            let mut right_envelopes = right
                .list_envelopes(folder, Default::default())
                .await
                .unwrap();
            right_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

            let mut right_cached_envelopes = right_cache
                .list_envelopes(folder, Default::default())
                .await
                .unwrap();
            right_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

            assert_eq!(left_envelopes, left_cached_envelopes);
            assert_eq!(right_envelopes, right_cached_envelopes);
            assert_eq!(left_envelopes, right_envelopes);
        }

        // check left emails content integrity

        let mut left_envelopes = left
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
        left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let ids = Id::multiple(left_envelopes.iter().map(|e| &e.id));
        let msgs = left.peek_messages(INBOX, &ids).await.unwrap();
        let msgs = msgs.to_vec();
        assert_eq!(3, msgs.len());
        assert_eq!("C", msgs[0].parsed().unwrap().body_text(0).unwrap());
        assert_eq!("B", msgs[1].parsed().unwrap().body_text(0).unwrap());
        assert_eq!("A", msgs[2].parsed().unwrap().body_text(0).unwrap());

        let mut left_envelopes = left
            .list_envelopes("Junk", Default::default())
            .await
            .unwrap();
        left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let ids = Id::multiple(left_envelopes.iter().map(|e| &e.id));
        let msgs = left.peek_messages("Junk", &ids).await.unwrap();
        let msgs = msgs.to_vec();
        assert_eq!(2, msgs.len());
        assert_eq!("E", msgs[0].parsed().unwrap().body_text(0).unwrap());
        assert_eq!("D", msgs[1].parsed().unwrap().body_text(0).unwrap());

        // remove messages and update flags from both side, sync again and
        // check integrity

        let mut left_envelopes = left
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
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

        let mut right_envelopes = right
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
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

        let mut left_envelopes = left
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
        left_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let mut left_cached_envelopes = left_cache
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
        left_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let mut right_envelopes = right
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
        right_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        let mut right_cached_envelopes = right_cache
            .list_envelopes(INBOX, Default::default())
            .await
            .unwrap();
        right_cached_envelopes.sort_by(|a, b| b.message_id.cmp(&a.message_id));

        assert!(report.folder.patch.is_empty());
        assert!(!report.email.patch.is_empty());
        assert_eq!(left_envelopes, left_cached_envelopes);
        assert_eq!(right_envelopes, right_cached_envelopes);
        assert_eq!(left_envelopes, right_envelopes);
    })
    .await
}
