#[cfg(feature = "notmuch-backend")]
#[test]
fn test_notmuch_backend() {
    use concat_with::concat_line;
    use mail_builder::MessageBuilder;
    use maildirpp::Maildir;
    use notmuch::Database;
    use pimalaya_email::{AccountConfig, Backend, Flag, Flags, NotmuchBackend, NotmuchConfig};
    use std::{collections::HashMap, env, fs, iter::FromIterator};

    env_logger::builder().is_test(true).init();

    // set up maildir folders and notmuch database

    let mdir = Maildir::from(env::temp_dir().join("himalaya-test-notmuch"));
    if let Err(_) = fs::remove_dir_all(mdir.path()) {}
    mdir.create_dirs().unwrap();

    let custom_mdir = Maildir::from(mdir.path().join("CustomMaildirFolder"));
    if let Err(_) = fs::remove_dir_all(custom_mdir.path()) {}
    custom_mdir.create_dirs().unwrap();

    Database::create(mdir.path()).unwrap();

    let config = AccountConfig {
        name: "account".into(),
        folder_aliases: HashMap::from_iter([
            ("inbox".into(), "".into()),
            ("custom".into(), "CustomMaildirFolder".into()),
        ]),
        ..AccountConfig::default()
    };

    let notmuch = NotmuchBackend::new(
        config.clone(),
        NotmuchConfig {
            db_path: mdir.path().to_owned(),
        },
    )
    .unwrap();

    // check that a message can be added
    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message custom!")
        .text_body("Plain message custom!")
        .write_to_vec()
        .unwrap();
    let flags = Flags::from_iter([Flag::Seen]);
    notmuch.add_email("custom", &email, &flags).unwrap();

    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    let flags = Flags::from_iter([Flag::custom("flag"), Flag::Seen]);
    let id = notmuch.add_email("inbox", &email, &flags).unwrap();

    // check that the added message exists
    let emails = notmuch.get_emails("inbox", vec![&id]).unwrap();
    let tpl = emails
        .to_vec()
        .first()
        .unwrap()
        .to_read_tpl(&config, |i| i.show_only_headers(["From", "To"]))
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
    let envelopes = notmuch.list_envelopes("custom", 0, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message custom!", envelope.subject);

    let envelopes = notmuch.list_envelopes("inbox", 0, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);

    // check that a flag can be added to the message
    let flags = Flags::from_iter([Flag::Flagged, Flag::Answered]);
    notmuch
        .add_flags("inbox", vec![&envelope.id], &flags)
        .unwrap();
    let envelopes = notmuch.list_envelopes("inbox", 0, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that the message flags can be changed
    let flags = Flags::from_iter([Flag::custom("flag"), Flag::Answered]);
    notmuch.set_flags("", vec![&envelope.id], &flags).unwrap();
    let envelopes = notmuch.list_envelopes("inbox", 0, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from the message
    let flags = Flags::from_iter([Flag::Answered]);
    notmuch
        .remove_flags("", vec![&envelope.id], &flags)
        .unwrap();
    let envelopes = notmuch.list_envelopes("inbox", 0, 0).unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be deleted
    notmuch.delete_emails("", vec![&id]).unwrap();
    assert!(notmuch.get_emails("inbox", vec![&id]).is_err());
}
