#[cfg(feature = "imap-backend")]
#[tokio::test]
async fn test_imap_backend() {
    use concat_with::concat_line;
    use mail_builder::MessageBuilder;
    use pimalaya_email::{
        account::{AccountConfig, PasswdConfig},
        backend::{BackendBuilder, BackendConfig, ImapAuthConfig, ImapConfig},
        email::{Flag, Tpl},
    };
    use pimalaya_secret::Secret;

    env_logger::builder().is_test(true).init();

    let config = AccountConfig {
        email_reading_decrypt_cmd: Some(
            "gpg --decrypt --quiet --recipient-file ./tests/keys/bob.key".into(),
        ),
        email_reading_verify_cmd: Some("gpg --verify --quiet".into()),
        backend: BackendConfig::Imap(ImapConfig {
            host: "127.0.0.1".into(),
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

    let imap_builder = BackendBuilder::new(config.clone());
    let mut imap = imap_builder.build().await.unwrap();

    // setting up folders

    for folder in imap.list_folders().await.unwrap().iter() {
        match folder.name.as_str() {
            "INBOX" => imap.purge_folder("INBOX").await.unwrap(),
            folder => imap.delete_folder(folder).await.unwrap(),
        }
    }

    imap.add_folder("Sent").await.unwrap();
    imap.add_folder("Trash").await.unwrap();
    imap.add_folder("Отправленные").await.unwrap();

    // checking that an email can be built and added
    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("subject")
        .text_body(concat_line!(
            "<#part type=text/plain>",
            "Hello, world!",
            "<#/part>",
        ));
    let email = Tpl::from(email.write_to_string().unwrap())
        .compile()
        .await
        .unwrap()
        .write_to_vec()
        .unwrap();

    let id = imap
        .add_email("Sent", &email, &("seen".into()))
        .await
        .unwrap()
        .to_string();

    // checking that the added email exists
    let emails = imap.get_emails("Sent", vec![&id]).await.unwrap();
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
        "Hello, world!",
        "",
    );

    assert_eq!(*tpl, expected_tpl);

    // checking that the envelope of the added email exists
    let sent = imap.list_envelopes("Sent", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!("alice@localhost", sent[0].from.addr);
    assert_eq!("subject", sent[0].subject);

    // checking that the email can be copied
    imap.copy_emails("Sent", "Отправленные", vec![&sent[0].id])
        .await
        .unwrap();
    let sent = imap.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());

    // checking that the email can be marked as deleted then expunged
    imap.mark_emails_as_deleted("Отправленные", vec![&sent_ru[0].id])
        .await
        .unwrap();
    let sent = imap.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());
    assert!(sent_ru[0].flags.contains(&Flag::Deleted));

    imap.expunge_folder("Отправленные").await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    assert_eq!(0, sent_ru.len());

    // checking that the email can be moved
    imap.move_emails("Sent", "Отправленные", vec![&sent[0].id])
        .await
        .unwrap();
    let sent = imap.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());

    // checking that the email can be deleted
    imap.delete_emails("Отправленные", vec![&sent_ru[0].id])
        .await
        .unwrap();
    let sent = imap.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, sent.len());
    assert_eq!(0, sent_ru.len());
    assert_eq!(1, trash.len());

    imap.delete_emails("Trash", vec![&trash[0].id])
        .await
        .unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, trash.len());
    assert!(trash[0].flags.contains(&Flag::Deleted));

    imap.expunge_folder("Trash").await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, trash.len());
}
