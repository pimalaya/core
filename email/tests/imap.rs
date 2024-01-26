use concat_with::concat_line;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::Id,
    flag::Flag,
    folder::{config::FolderConfig, INBOX, SENT},
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
};
use mml::MmlCompilerBuilder;
use secret::Secret;
use std::{collections::HashMap, sync::Arc};

#[tokio::test]
async fn test_imap_features() {
    env_logger::builder().is_test(true).init();

    let account_config = Arc::new(AccountConfig {
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([(SENT.into(), "[Gmail]/Sent".into())])),
            ..Default::default()
        }),
        ..Default::default()
    });

    let imap_config = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: 3143,
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..Default::default()
    });

    let imap_ctx = ImapContextBuilder::new(imap_config.clone());
    let imap = BackendBuilder::new(account_config.clone(), imap_ctx)
        .build()
        .await
        .unwrap();

    // setting up folders

    for folder in imap.list_folders().await.unwrap().iter() {
        if folder.is_inbox() {
            imap.purge_folder(INBOX).await.unwrap()
        } else {
            imap.delete_folder(&folder.name).await.unwrap()
        }
    }

    imap.add_folder("[Gmail]/Sent").await.unwrap();
    imap.add_folder("Trash").await.unwrap();
    imap.add_folder("Отправленные").await.unwrap();

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

    let id = imap
        .add_message_with_flag(SENT, &email, Flag::Seen)
        .await
        .unwrap();

    // checking that the added email exists
    let msgs = imap.get_messages(SENT, &id.into()).await.unwrap();

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
    let sent = imap.list_envelopes(SENT, 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!("alice@localhost", sent[0].from.addr);
    assert_eq!("subject", sent[0].subject);

    // checking that the email can be copied
    imap.copy_messages(SENT, "Отправленные", &Id::single(&sent[0].id))
        .await
        .unwrap();
    let sent = imap.list_envelopes(SENT, 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());

    // checking that the email can be marked as deleted then expunged
    imap.add_flag("Отправленные", &Id::single(&sent_ru[0].id), Flag::Deleted)
        .await
        .unwrap();
    let sent = imap.list_envelopes(SENT, 0, 0).await.unwrap();
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
    imap.move_messages(SENT, "Отправленные", &Id::single(&sent[0].id))
        .await
        .unwrap();
    let sent = imap.list_envelopes(SENT, 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, sent.len());
    assert_eq!(1, sent_ru.len());
    assert_eq!(0, trash.len());

    // checking that the email can be deleted
    imap.delete_messages("Отправленные", &Id::single(&sent_ru[0].id))
        .await
        .unwrap();
    let sent = imap.list_envelopes(SENT, 0, 0).await.unwrap();
    let sent_ru = imap.list_envelopes("Отправленные", 0, 0).await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, sent.len());
    assert_eq!(0, sent_ru.len());
    assert_eq!(1, trash.len());

    imap.delete_messages("Trash", &Id::single(&trash[0].id))
        .await
        .unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, trash.len());
    assert!(trash[0].flags.contains(&Flag::Deleted));

    imap.expunge_folder("Trash").await.unwrap();
    let trash = imap.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, trash.len());
}
