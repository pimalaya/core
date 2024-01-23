use async_trait::async_trait;
use concat_with::concat_line;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::{
        BackendBuilder, BackendBuilderV2, BackendContextBuilder, BackendContextBuilderV2,
        BackendContextMapper, BackendConvertFeature,
    },
    envelope::{flag::add::imap::AddImapFlags, list::imap::ListImapEnvelopes, Id},
    flag::Flag,
    folder::{
        add::imap::AddImapFolder,
        config::FolderConfig,
        delete::imap::DeleteImapFolder,
        expunge::imap::ExpungeImapFolder,
        list::{imap::ListImapFolders, ListFolders},
        purge::imap::PurgeImapFolder,
        INBOX, SENT,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder, ImapContextSync,
    },
    message::{
        add::imap::AddImapMessage, copy::imap::CopyImapMessages, get::imap::GetImapMessages,
        move_::imap::MoveImapMessages,
    },
    smtp::{
        config::{SmtpAuthConfig, SmtpConfig, SmtpEncryptionKind},
        SmtpContextBuilder, SmtpContextSync,
    },
    Result,
};
use mml::MmlCompilerBuilder;
use secret::Secret;
use std::{collections::HashMap, sync::Arc};

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
    let smtp_config = SmtpConfig {
        host: "localhost".into(),
        port: 3025,
        encryption: Some(SmtpEncryptionKind::None),
        login: "alice@localhost".into(),
        auth: SmtpAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..Default::default()
    };

    struct MyContext {
        imap: ImapContextSync,
        smtp: SmtpContextSync,
    }

    impl BackendContextMapper<ImapContextSync> for MyContext {
        fn map_context(&self) -> &ImapContextSync {
            &self.imap
        }
    }

    impl BackendContextMapper<SmtpContextSync> for MyContext {
        fn map_context(&self) -> &SmtpContextSync {
            &self.smtp
        }
    }

    #[derive(Clone)]
    struct MyContextBuilder {
        imap: ImapContextBuilder,
        smtp: SmtpContextBuilder,
    }

    impl<C1: BackendContextMapper<C2>, C2: Send + 'static> BackendConvertFeature<C1, C2>
        for MyContextBuilder
    {
    }

    #[async_trait]
    impl BackendContextBuilderV2 for MyContextBuilder {
        type Context = MyContext;

        #[cfg(feature = "folder-list")]
        fn list_folders_builder(
            &self,
        ) -> Option<Arc<dyn Fn(&Self::Context) -> Option<Box<dyn ListFolders>> + Send + Sync>>
        {
            self.convert_feature(self.imap.list_folders_builder())
        }

        async fn build(self, account_config: &AccountConfig) -> Result<Self::Context> {
            Ok(MyContext {
                imap: BackendContextBuilderV2::build(self.imap, account_config).await?,
                smtp: self.smtp.build(account_config).await?,
            })
        }
    }

    let ctx_builder = ImapContextBuilder::new(imap_config.clone())
        .with_prebuilt_credentials()
        .await
        .unwrap();
    let backend = BackendBuilder::new(account_config.clone(), ctx_builder.clone())
        .with_add_folder(AddImapFolder::some_new_boxed)
        .with_list_folders(ListImapFolders::some_new_boxed)
        .with_expunge_folder(ExpungeImapFolder::some_new_boxed)
        .with_purge_folder(PurgeImapFolder::some_new_boxed)
        .with_delete_folder(DeleteImapFolder::some_new_boxed)
        .with_list_envelopes(ListImapEnvelopes::some_new_boxed)
        .with_add_flags(AddImapFlags::some_new_boxed)
        .with_add_message(AddImapMessage::some_new_boxed)
        .with_get_messages(GetImapMessages::some_new_boxed)
        .with_copy_messages(CopyImapMessages::some_new_boxed)
        .with_move_messages(MoveImapMessages::some_new_boxed)
        .build()
        .await
        .unwrap();
    let backend_v2 = BackendBuilderV2::new(MyContextBuilder {
        imap: ImapContextBuilder::new(imap_config.clone()),
        smtp: SmtpContextBuilder::new(smtp_config.clone()),
    })
    .build(account_config.clone())
    .await
    .unwrap();

    // setting up folders

    for folder in backend_v2.list_folders().await.unwrap().iter() {
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
