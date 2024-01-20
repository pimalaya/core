// use concat_with::concat_line;
// use email::{
//     account::config::AccountConfig,
//     backend::{Backend, NotmuchBackend, NotmuchConfig},
//     email::{Flag, Flags},
//     folder::config::FolderConfig,
// };
// use mail_builder::MessageBuilder;
// use maildirpp::Maildir;
// use notmuch::Database;
// use std::{collections::HashMap, env, fs, iter::FromIterator};
// use tempfile::tempdir;

// #[tokio::test(flavor = "multi_thread")]
// async fn test_notmuch_features() {
//     env_logger::builder().is_test(true).init();

//     // set up maildir folders and notmuch database

//     let mdir: Maildir = tempdir().unwrap().path().to_owned().into();
//     if let Err(_) = fs::remove_dir_all(mdir.path()) {}
//     mdir.create_dirs().unwrap();

//     let custom_mdir: Maildir = mdir.path().join("CustomMaildirFolder").into();
//     if let Err(_) = fs::remove_dir_all(custom_mdir.path()) {}
//     custom_mdir.create_dirs().unwrap();

//     Database::create(mdir.path()).unwrap();

//     let config = AccountConfig {
//         name: "account".into(),
//         folder: Some(FolderConfig {
//             aliases: Some(HashMap::from_iter([
//                 ("inbox".into(), "folder:\"\"".into()),
//                 ("custom".into(), "folder:\"CustomMaildirFolder\"".into()),
//             ])),
//             ..Default::default()
//         }),
//         ..Default::default()
//     };

//     let notmuch_config = NotmuchConfig {
//         db_path: mdir.path().to_owned(),
//     };

//     let notmuch_ctx = NotmuchSessionBuilder::new(config.clone(), mdir_config);
//     let mut notmuch = NotmuchBackend::new(
//         config.clone(),
//         NotmuchConfig {
//             db_path: mdir.path().to_owned(),
//         },
//     )
//     .unwrap();

//     // check that a message can be added
//     let email = MessageBuilder::new()
//         .from("alice@localhost")
//         .to("bob@localhost")
//         .subject("Plain message custom!")
//         .text_body("Plain message custom!")
//         .write_to_vec()
//         .unwrap();
//     let flags = Flags::from_iter([Flag::Seen]);
//     notmuch.add_email("custom", &email, &flags).await.unwrap();

//     let email = MessageBuilder::new()
//         .from("alice@localhost")
//         .to("bob@localhost")
//         .subject("Plain message!")
//         .text_body("Plain message!")
//         .write_to_vec()
//         .unwrap();
//     let flags = Flags::from_iter([Flag::custom("flag"), Flag::Seen]);
//     let id = notmuch.add_email("inbox", &email, &flags).await.unwrap();

//     // check that the added message exists
//     let emails = notmuch.get_emails("inbox", vec![&id]).await.unwrap();
//     let tpl = emails
//         .to_vec()
//         .first()
//         .unwrap()
//         .to_read_tpl(&config, |i| i.with_show_only_headers(["From", "To"]))
//         .await
//         .unwrap();
//     let expected_tpl = concat_line!(
//         "From: alice@localhost",
//         "To: bob@localhost",
//         "",
//         "Plain message!",
//         "",
//     );

//     assert_eq!(tpl, expected_tpl);

//     // check that the envelope of the added message exists
//     let envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();
//     let envelope = envelopes.first().unwrap();
//     assert_eq!(1, envelopes.len());
//     assert_eq!("alice@localhost", envelope.from.addr);
//     assert_eq!("Plain message custom!", envelope.subject);

//     let envelopes = notmuch.list_envelopes("inbox", 0, 0).await.unwrap();
//     let envelope = envelopes.first().unwrap();
//     assert_eq!(1, envelopes.len());
//     assert_eq!("alice@localhost", envelope.from.addr);
//     assert_eq!("Plain message!", envelope.subject);

//     let envelopes = notmuch
//         .search_envelopes("inbox", "", "", 0, 0)
//         .await
//         .unwrap();
//     let envelope = envelopes.first().unwrap();
//     assert_eq!(1, envelopes.len());
//     assert_eq!("alice@localhost", envelope.from.addr);
//     assert_eq!("Plain message!", envelope.subject);

//     // check that a flag can be added to the message
//     let flags = Flags::from_iter([Flag::Flagged, Flag::Answered]);
//     notmuch
//         .add_flags("inbox", vec![&envelope.id], &flags)
//         .await
//         .unwrap();
//     let envelopes = notmuch.list_envelopes("inbox", 0, 0).await.unwrap();
//     let envelope = envelopes.first().unwrap();
//     assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
//     assert!(envelope.flags.contains(&Flag::Seen));
//     assert!(envelope.flags.contains(&Flag::Flagged));
//     assert!(envelope.flags.contains(&Flag::Answered));

//     // check that the message flags can be changed
//     let flags = Flags::from_iter([Flag::custom("flag"), Flag::Answered]);
//     notmuch
//         .set_flags("", vec![&envelope.id], &flags)
//         .await
//         .unwrap();
//     let envelopes = notmuch.list_envelopes("inbox", 0, 0).await.unwrap();
//     let envelope = envelopes.first().unwrap();
//     assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
//     assert!(!envelope.flags.contains(&Flag::Seen));
//     assert!(!envelope.flags.contains(&Flag::Flagged));
//     assert!(envelope.flags.contains(&Flag::Answered));

//     // check that a flag can be removed from the message
//     let flags = Flags::from_iter([Flag::Answered]);
//     notmuch
//         .remove_flags("", vec![&envelope.id], &flags)
//         .await
//         .unwrap();
//     let envelopes = notmuch.list_envelopes("inbox", 0, 0).await.unwrap();
//     let envelope = envelopes.first().unwrap();
//     assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
//     assert!(!envelope.flags.contains(&Flag::Seen));
//     assert!(!envelope.flags.contains(&Flag::Flagged));
//     assert!(!envelope.flags.contains(&Flag::Answered));

//     // check that the message can be deleted
//     notmuch.delete_emails("", vec![&id]).await.unwrap();
//     assert!(notmuch.get_emails("inbox", vec![&id]).await.is_err());
// }
