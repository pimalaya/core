use concat_with::concat_line;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::{flag::add::imap::AddImapFlags, list::imap::ListImapEnvelopes, Id},
    flag::Flag,
    folder::{
        add::imap::AddImapFolder, config::FolderConfig, delete::imap::DeleteImapFolder,
        expunge::imap::ExpungeImapFolder, list::imap::ListImapFolders,
        purge::imap::PurgeImapFolder, INBOX, SENT,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
    message::{
        add::imap::AddImapMessage, copy::imap::CopyImapMessages, get::imap::GetImapMessages,
        move_::imap::MoveImapMessages,
    },
};
use mml::MmlCompilerBuilder;
use secret::Secret;
use std::collections::HashMap;

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
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..Default::default()
    };

    let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config)
        .with_prebuilt_credentials()
        .await
        .unwrap();
    let backend_builder = BackendBuilder::new(account_config.clone(), imap_ctx)
        .with_add_folder(|ctx| Some(AddImapFolder::new_boxed(ctx.clone())))
        .with_list_folders(|ctx| Some(ListImapFolders::new_boxed(ctx.clone())))
        .with_expunge_folder(|ctx| Some(ExpungeImapFolder::new_boxed(ctx.clone())))
        .with_purge_folder(|ctx| Some(PurgeImapFolder::new_boxed(ctx.clone())))
        .with_delete_folder(|ctx| Some(DeleteImapFolder::new_boxed(ctx.clone())))
        .with_list_envelopes(|ctx| Some(ListImapEnvelopes::new_boxed(ctx.clone())))
        .with_add_flags(|ctx| Some(AddImapFlags::new_boxed(ctx.clone())))
        .with_add_message(|ctx| Some(AddImapMessage::new_boxed(ctx.clone())))
        .with_get_messages(|ctx| Some(GetImapMessages::new_boxed(ctx.clone())))
        .with_copy_messages(|ctx| Some(CopyImapMessages::new_boxed(ctx.clone())))
        .with_move_messages(|ctx| Some(MoveImapMessages::new_boxed(ctx.clone())));
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
        .add_message_with_flag(SENT, &email, Flag::Seen)
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
