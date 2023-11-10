#[cfg(feature = "imap-backend")]
#[tokio::test]
async fn test_imap_features() {
    use concat_with::concat_line;
    use email::{
        account::{AccountConfig, PasswdConfig},
        backend::{BackendBuilderV2, BackendConfig, ImapAuthConfig, ImapConfig},
        email::{
            envelope::{flag::add::imap::AddFlagsImap, list::imap::ListEnvelopesImap, Id},
            message::{
                add_raw_with_flags::imap::AddRawMessageWithFlagsImap, copy::imap::CopyMessagesImap,
                get::imap::GetMessagesImap, move_::imap::MoveMessagesImap,
            },
            Flag,
        },
        folder::{
            add::imap::AddFolderImap, delete::imap::DeleteFolderImap,
            expunge::imap::ExpungeFolderImap, list::imap::ListFoldersImap,
            purge::imap::PurgeFolderImap,
        },
        imap::ImapSessionBuilder,
        prelude::*,
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

    let account_config = AccountConfig {
        backend: BackendConfig::Imap(imap_config.clone()),
        ..AccountConfig::default()
    };

    let imap_ctx = ImapSessionBuilder::new(account_config.clone(), imap_config);
    let backend_builder = BackendBuilderV2::new(account_config.clone(), imap_ctx)
        .with_add_folder(AddFolderImap::new)
        .with_list_folders(ListFoldersImap::new)
        .with_expunge_folder(ExpungeFolderImap::new)
        .with_purge_folder(PurgeFolderImap::new)
        .with_delete_folder(DeleteFolderImap::new)
        .with_list_envelopes(ListEnvelopesImap::new)
        .with_add_flags(AddFlagsImap::new)
        .with_get_messages(GetMessagesImap::new)
        .with_add_raw_message_with_flags(AddRawMessageWithFlagsImap::new)
        .with_copy_messages(CopyMessagesImap::new)
        .with_move_messages(MoveMessagesImap::new);
    let backend = backend_builder.build().await.unwrap();

    // setting up folders

    for folder in backend.list_folders().await.unwrap().iter() {
        match folder.name.as_str() {
            "INBOX" => backend.purge_folder("INBOX").await.unwrap(),
            folder => backend.delete_folder(folder).await.unwrap(),
        }
    }

    backend.add_folder("Sent").await.unwrap();
    backend.add_folder("Trash").await.unwrap();
    backend.add_folder("Отправленные").await.unwrap();

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

    let id = backend
        .add_raw_message_with_flag("Sent", &email, Flag::Seen)
        .await
        .unwrap();

    // checking that the added email exists
    let msgs = backend.get_messages("Sent", &id.into()).await.unwrap();

    let tpl = msgs
        .to_vec()
        .first()
        .unwrap()
        .to_read_tpl(&account_config, |i| {
            i.with_show_only_headers(["From", "To"])
        })
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
    let sent = backend.list_envelopes("Sent", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!("alice@localhost", sent[0].from.addr);
    assert_eq!("subject", sent[0].subject);

    // checking that the email can be copied
    backend
        .copy_messages("Sent", "Отправленные", &Id::single(&sent[0].id))
        .await
        .unwrap();
    let sent = backend.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = backend.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = backend.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());

    // checking that the email can be marked as deleted then expunged
    backend
        .add_flag("Отправленные", &Id::single(&sent_ru[0].id), Flag::Deleted)
        .await
        .unwrap();
    let sent = backend.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = backend.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = backend.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());
    assert!(sent_ru[0].flags.contains(&Flag::Deleted));

    backend.expunge_folder("Отправленные").await.unwrap();
    let sent_ru = backend.list_envelopes("Отправленные", 0, 0).await.unwrap();
    assert_eq!(0, sent_ru.len());

    // checking that the email can be moved
    backend
        .move_messages("Sent", "Отправленные", &Id::single(&sent[0].id))
        .await
        .unwrap();
    let sent = backend.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = backend.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = backend.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());

    // checking that the email can be deleted
    backend
        .delete_messages("Отправленные", &Id::single(&sent_ru[0].id))
        .await
        .unwrap();
    let sent = backend.list_envelopes("Sent", 0, 0).await.unwrap();
    let sent_ru = backend.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = backend.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, sent.len());
    assert_eq!(0, sent_ru.len());
    assert_eq!(1, trash.len());

    backend
        .delete_messages("Trash", &Id::single(&trash[0].id))
        .await
        .unwrap();
    let trash = backend.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, trash.len());
    assert!(trash[0].flags.contains(&Flag::Deleted));

    backend.expunge_folder("Trash").await.unwrap();
    let trash = backend.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, trash.len());
}
