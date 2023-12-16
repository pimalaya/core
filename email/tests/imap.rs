use std::collections::HashMap;

use concat_with::concat_line;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::{flag::add::imap::AddFlagsImap, list::imap::ListEnvelopesImap, Id},
    flag::Flag,
    folder::{
        add::imap::AddFolderImap, config::FolderConfig, delete::imap::DeleteFolderImap,
        expunge::imap::ExpungeFolderImap, list::imap::ListFoldersImap,
        purge::imap::PurgeFolderImap, INBOX, SENT,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapSessionBuilder,
    },
    message::{
        add_raw_with_flags::imap::AddRawMessageWithFlagsImap, copy::imap::CopyMessagesImap,
        get::imap::GetMessagesImap, move_::imap::MoveMessagesImap,
    },
};
use mml::MmlCompilerBuilder;
use secret::Secret;

#[tokio::test]
async fn test_imap_features() {
    env_logger::builder().is_test(true).init();

    let account_config = AccountConfig {
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([(SENT.into(), "[Gmail]/Sent".into())])),
            ..Default::default()
        }),
        ..Default::default()
    };
    let imap_config = ImapConfig {
        host: "localhost".into(),
        port: 3143,
        ssl: Some(false),
        starttls: Some(false),
        insecure: Some(true),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig {
            passwd: Secret::new_raw("password"),
        }),
        ..Default::default()
    };

    let imap_ctx = ImapSessionBuilder::new(account_config.clone(), imap_config)
        .with_prebuilt_credentials()
        .await
        .unwrap();
    let backend_builder = BackendBuilder::new(account_config.clone(), imap_ctx)
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
        if folder.is_inbox() {
            backend.purge_folder(INBOX).await.unwrap()
        } else {
            backend.delete_folder(&folder.name).await.unwrap()
        }
    }

    backend.add_folder("[Gmail]/Sent").await.unwrap();
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
        .add_raw_message_with_flag(SENT, &email, Flag::Seen)
        .await
        .unwrap();

    // checking that the added email exists
    let msgs = backend.get_messages(SENT, &id.into()).await.unwrap();

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
    let sent = backend.list_envelopes(SENT, 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!("alice@localhost", sent[0].from.addr);
    assert_eq!("subject", sent[0].subject);

    // checking that the email can be copied
    backend
        .copy_messages(SENT, "Отправленные", &Id::single(&sent[0].id))
        .await
        .unwrap();
    let sent = backend.list_envelopes(SENT, 0, 0).await.unwrap();
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
    let sent = backend.list_envelopes(SENT, 0, 0).await.unwrap();
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
        .move_messages(SENT, "Отправленные", &Id::single(&sent[0].id))
        .await
        .unwrap();
    let sent = backend.list_envelopes(SENT, 0, 0).await.unwrap();
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
    let sent = backend.list_envelopes(SENT, 0, 0).await.unwrap();
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
