use std::{collections::HashMap, sync::Arc};

use concat_with::concat_line;
use email::{
    account::config::{passwd::PasswordConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::{list::ListEnvelopes, Id},
    flag::{add::AddFlags, Flag},
    folder::{add::AddFolder, config::FolderConfig, expunge::ExpungeFolder, SENT},
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapContextBuilder,
    },
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        r#move::MoveMessages,
    },
    tls::Encryption,
};
use email_testing_server::with_email_testing_server;
use mml::MmlCompilerBuilder;
use secret::Secret;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_imap_features() {
    with_email_testing_server(|ports| async move {
        let account_config = Arc::new(AccountConfig {
            folder: Some(FolderConfig {
                aliases: Some(HashMap::from_iter([(SENT.into(), "[Gmail]/Sent".into())])),
                ..Default::default()
            }),
            ..Default::default()
        });

        let imap_config = Arc::new(ImapConfig {
            host: "localhost".into(),
            port: ports.imap,
            encryption: Some(Encryption::None),
            login: "bob".into(),
            auth: ImapAuthConfig::Password(PasswordConfig(Secret::new_raw("password"))),
            ..Default::default()
        });

        let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config.clone());
        let imap = BackendBuilder::new(account_config.clone(), imap_ctx)
            .build()
            .await
            .unwrap();

        // setting up folders

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
        let compiler = MmlCompilerBuilder::new().build(tpl).unwrap();
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

        assert_eq!(*tpl, expected_tpl);

        // checking that the envelope of the added email exists
        let sent = imap.list_envelopes(SENT, Default::default()).await.unwrap();
        assert_eq!(1, sent.len());
        assert_eq!("alice@localhost", sent[0].from.addr);
        assert_eq!("subject", sent[0].subject);

        // checking that the email can be copied
        imap.copy_messages(SENT, "Отправленные", &Id::single(&sent[0].id))
            .await
            .unwrap();
        let sent = imap.list_envelopes(SENT, Default::default()).await.unwrap();
        let sent_ru = imap
            .list_envelopes("Отправленные", Default::default())
            .await
            .unwrap();
        let trash = imap
            .list_envelopes("Trash", Default::default())
            .await
            .unwrap();
        assert_eq!(1, sent.len());
        assert_eq!(1, sent_ru.len());
        assert_eq!(0, trash.len());

        // checking that the email can be marked as deleted then expunged
        imap.add_flag("Отправленные", &Id::single(&sent_ru[0].id), Flag::Deleted)
            .await
            .unwrap();
        let sent = imap.list_envelopes(SENT, Default::default()).await.unwrap();
        let sent_ru = imap
            .list_envelopes("Отправленные", Default::default())
            .await
            .unwrap();
        let trash = imap
            .list_envelopes("Trash", Default::default())
            .await
            .unwrap();
        assert_eq!(1, sent.len());
        assert_eq!(1, sent_ru.len());
        assert_eq!(0, trash.len());
        assert!(sent_ru[0].flags.contains(&Flag::Deleted));

        imap.expunge_folder("Отправленные").await.unwrap();
        let sent_ru = imap
            .list_envelopes("Отправленные", Default::default())
            .await
            .unwrap();
        assert_eq!(0, sent_ru.len());

        // checking that the email can be moved
        imap.move_messages(SENT, "Отправленные", &Id::single(&sent[0].id))
            .await
            .unwrap();
        let sent = imap.list_envelopes(SENT, Default::default()).await.unwrap();
        let sent_ru = imap
            .list_envelopes("Отправленные", Default::default())
            .await
            .unwrap();
        let trash = imap
            .list_envelopes("Trash", Default::default())
            .await
            .unwrap();
        assert_eq!(0, sent.len());
        assert_eq!(1, sent_ru.len());
        assert_eq!(0, trash.len());

        // checking that the email can be deleted
        imap.delete_messages("Отправленные", &Id::single(&sent_ru[0].id))
            .await
            .unwrap();
        let sent = imap.list_envelopes(SENT, Default::default()).await.unwrap();
        let sent_ru = imap
            .list_envelopes("Отправленные", Default::default())
            .await
            .unwrap();
        let trash = imap
            .list_envelopes("Trash", Default::default())
            .await
            .unwrap();
        assert_eq!(0, sent.len());
        assert_eq!(0, sent_ru.len());
        assert_eq!(1, trash.len());

        imap.delete_messages("Trash", &Id::single(&trash[0].id))
            .await
            .unwrap();
        let trash = imap
            .list_envelopes("Trash", Default::default())
            .await
            .unwrap();
        assert_eq!(1, trash.len());
        assert!(trash[0].flags.contains(&Flag::Deleted));

        imap.expunge_folder("Trash").await.unwrap();
        let trash = imap
            .list_envelopes("Trash", Default::default())
            .await
            .unwrap();
        assert_eq!(0, trash.len());
    })
    .await
}
