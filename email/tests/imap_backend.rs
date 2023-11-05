#[cfg(feature = "imap-backend")]
#[tokio::test]
async fn test_imap_backend() {
    use concat_with::concat_line;
    use email::{
        account::{AccountConfig, PasswdConfig},
        backend::{
            prelude::*, BackendBuilder, BackendBuilderV2, BackendConfig, ImapAuthConfig, ImapConfig,
        },
        email::{add::imap::AddRawImapMessageWithFlags, Flag},
        folder::{
            add::imap::AddImapFolder, delete::imap::DeleteImapFolder,
            expunge::imap::ExpungeImapFolder, list::imap::ListImapFolders,
            purge::imap::PurgeImapFolder,
        },
        imap::ImapSessionBuilder,
    };
    use mml::MmlCompilerBuilder;
    use secret::Secret;

    env_logger::builder().is_test(true).init();

    let imap_config = ImapConfig {
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
    };

    let config = AccountConfig {
        backend: BackendConfig::Imap(imap_config.clone()),
        ..AccountConfig::default()
    };

    let backend_context_v2 = ImapSessionBuilder::new(config.clone(), imap_config);
    let backend_builder_v2 = BackendBuilderV2::new(backend_context_v2)
        .with_add_folder(AddImapFolder::new)
        .with_list_folders(ListImapFolders::new)
        .with_expunge_folder(ExpungeImapFolder::new)
        .with_purge_folder(PurgeImapFolder::new)
        .with_delete_folder(DeleteImapFolder::new)
        .with_add_email(AddRawImapMessageWithFlags::new);
    let backend_v2 = backend_builder_v2.build().await.unwrap();

    let imap_builder = BackendBuilder::new(config.clone());
    let mut imap = imap_builder.build().await.unwrap();

    // setting up folders

    for folder in backend_v2.list_folders().await.unwrap().iter() {
        match folder.name.as_str() {
            "INBOX" => backend_v2.purge_folder("INBOX").await.unwrap(),
            folder => backend_v2.delete_folder(folder).await.unwrap(),
        }
    }

    backend_v2.add_folder("Sent").await.unwrap();
    backend_v2.add_folder("Trash").await.unwrap();
    backend_v2.add_folder("Отправленные").await.unwrap();

    // checking that an email can be built and added
    let tpl = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain>",
        "Hello, world!",
        "<#/part>",
    );
    let compiler = MmlCompilerBuilder::new().build(&tpl).unwrap();
    let email = compiler.compile().await.unwrap().into_vec().unwrap();

    let id = backend_v2
        .add_raw_message_with_flags("Sent", &email, &("seen".into()))
        .await
        .unwrap()
        .to_string();

    // checking that the added email exists
    let emails = imap.get_emails("Sent", vec![&id]).await.unwrap();
    let tpl = emails
        .to_vec()
        .first()
        .unwrap()
        .to_read_tpl(&config, |i| i.with_show_only_headers(["From", "To"]))
        .await
        .unwrap();
    let expected_tpl = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "",
        "Hello, world!",
        "",
    );

    assert_eq!(tpl, expected_tpl);

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

    backend_v2.expunge_folder("Отправленные").await.unwrap();
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

    backend_v2.expunge_folder("Trash").await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, trash.len());
}
