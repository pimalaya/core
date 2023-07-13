#[tokio::test]
async fn maildir_backend() {
    use concat_with::concat_line;
    use mail_builder::MessageBuilder;
    use maildirpp::Maildir;
    use pimalaya_email::{
        account::AccountConfig,
        backend::{Backend, MaildirBackend, MaildirConfig},
        email::{Flag, Flags},
    };
    use std::{collections::HashMap, fs, iter::FromIterator};
    use tempfile::tempdir;

    env_logger::builder().is_test(true).init();

    // set up maildir folders

    let mdir: Maildir = tempdir().unwrap().path().to_owned().into();
    if let Err(_) = fs::remove_dir_all(mdir.path()) {}
    mdir.create_dirs().unwrap();

    let mdir_sub: Maildir = mdir.path().join(".Subdir").into();
    if let Err(_) = fs::remove_dir_all(mdir_sub.path()) {}
    mdir_sub.create_dirs().unwrap();

    let mdir_trash = Maildir::from(mdir.path().join(".Trash"));
    if let Err(_) = fs::remove_dir_all(mdir_trash.path()) {}
    mdir_trash.create_dirs().unwrap();

    let config = AccountConfig {
        name: "account".into(),
        folder_aliases: HashMap::from_iter([
            ("subdir".into(), "Subdir".into()),
            (
                "abs-subdir".into(),
                mdir.path().join(".Subdir").to_string_lossy().into(),
            ),
        ]),
        ..AccountConfig::default()
    };

    let mdir_path = mdir.path().to_owned();
    let mut mdir = MaildirBackend::new(
        config.clone(),
        MaildirConfig {
            root_dir: mdir_path.clone(),
        },
    )
    .unwrap();

    let mut submdir = MaildirBackend::new(
        config.clone(),
        MaildirConfig {
            root_dir: mdir_path.clone(),
        },
    )
    .unwrap();

    // check that a message can be built and added
    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    let flags = Flags::from_iter([Flag::Seen]);
    let id = mdir.add_email("INBOX", &email, &flags).await.unwrap();

    // check that the added message exists
    let emails = mdir.get_emails("INBOX", vec![&id]).await.unwrap();
    let tpl = emails
        .to_vec()
        .first()
        .unwrap()
        .to_read_tpl(&config, |i| i.show_only_headers(["From", "To"]))
        .await
        .unwrap();
    let expected_tpl = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "",
        "Plain message!",
        "",
    );

    assert_eq!(*tpl, expected_tpl);

    // check that the envelope of the added message exists
    let envelopes = mdir.list_envelopes("INBOX", 10, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);

    // check that a flag can be added to the message
    let flags = Flags::from_iter([Flag::Flagged]);
    mdir.add_flags("INBOX", vec![&envelope.id], &flags)
        .await
        .unwrap();
    let envelopes = mdir.list_envelopes("INBOX", 1, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));

    // check that the message flags can be changed
    let flags = Flags::from_iter([Flag::Answered]);
    mdir.set_flags("INBOX", vec![&envelope.id], &flags)
        .await
        .unwrap();
    let envelopes = mdir.list_envelopes("INBOX", 1, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from the message
    let flags = Flags::from_iter([Flag::Answered]);
    mdir.remove_flags("INBOX", vec![&envelope.id], &flags)
        .await
        .unwrap();
    let envelopes = mdir.list_envelopes("INBOX", 1, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be copied
    mdir.copy_emails("INBOX", "subdir", vec![&envelope.id])
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let abs_subdir = mdir.list_envelopes("abs-subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(1, abs_subdir.len());
    assert_eq!(0, trash.len());
    assert!(mdir.get_emails("INBOX", vec![&inbox[0].id]).await.is_ok());
    assert!(mdir.get_emails("subdir", vec![&subdir[0].id]).await.is_ok());
    assert!(submdir
        .get_emails("INBOX", vec![&subdir[0].id])
        .await
        .is_ok());

    // check that the email can be marked as deleted then expunged
    mdir.mark_emails_as_deleted("subdir", vec![&subdir[0].id])
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let abs_subdir = mdir.list_envelopes("abs-subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(1, abs_subdir.len());
    assert_eq!(0, trash.len());

    mdir.expunge_folder("subdir").await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let abs_subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    assert_eq!(0, subdir.len());
    assert_eq!(0, abs_subdir.len());

    // check that the message can be moved
    mdir.move_emails("INBOX", "subdir", vec![&envelope.id])
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(0, trash.len());

    // check that the message can be deleted
    mdir.delete_emails("subdir", vec![&subdir[0].id])
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, inbox.len());
    assert_eq!(0, subdir.len());
    assert_eq!(1, trash.len());

    mdir.delete_emails("Trash", vec![&trash[0].id])
        .await
        .unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, trash.len());
    assert!(trash[0].flags.contains(&Flag::Deleted));

    mdir.expunge_folder("Trash").await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, trash.len());
}
