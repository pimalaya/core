use async_trait::async_trait;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend_v2::{
        context::BackendContextBuilder,
        feature::BackendFeature,
        macros::BackendContextV2,
        mapper::{BackendContextBuilderMapper, SomeBackendContextBuilderMapper},
        pool::BackendPool,
        BackendBuilder,
    },
    folder::{
        config::FolderConfig,
        list::{imap::ListImapFolders, ListFolders},
        Folder, FolderKind, Folders, SENT,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder, ImapContextSync,
    },
    smtp::{
        config::{SmtpAuthConfig, SmtpConfig, SmtpEncryptionKind},
        SmtpContextBuilder, SmtpContextSync,
    },
    Result,
};
use secret::Secret;
use std::{collections::HashMap, ops::Deref, sync::Arc};

#[tokio::test]
async fn test_backend_v2() {
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

    let smtp_config = Arc::new(SmtpConfig {
        host: "localhost".into(),
        port: 3025,
        encryption: Some(SmtpEncryptionKind::None),
        login: "alice@localhost".into(),
        auth: SmtpAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..Default::default()
    });

    // TEST DYNAMIC BACKEND

    // 1. define custom context

    #[derive(BackendContextV2)]
    struct MyContext {
        imap: Option<ImapContextSync>,
        smtp: Option<SmtpContextSync>,
    }

    // 2. implement as refs (could be auto-implemented by macros)

    impl AsRef<Option<ImapContextSync>> for MyContext {
        fn as_ref(&self) -> &Option<ImapContextSync> {
            &self.imap
        }
    }

    impl AsRef<Option<SmtpContextSync>> for MyContext {
        fn as_ref(&self) -> &Option<SmtpContextSync> {
            &self.smtp
        }
    }

    // 3. define custom context builder

    #[derive(Clone)]
    struct MyContextBuilder {
        imap: Option<ImapContextBuilder>,
        smtp: Option<SmtpContextBuilder>,
    }

    // 4. implement backend context builder

    #[async_trait]
    impl BackendContextBuilder for MyContextBuilder {
        type Context = MyContext;

        fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
            self.list_folders_with_some(&self.imap)
        }

        async fn build(self) -> Result<Self::Context> {
            let imap = match self.imap {
                Some(imap) => Some(imap.build().await?),
                None => None,
            };

            let smtp = match self.smtp {
                Some(smtp) => Some(smtp.build().await?),
                None => None,
            };

            Ok(MyContext { imap, smtp })
        }
    }

    // 5. plug all together

    let ctx_builder = MyContextBuilder {
        imap: Some(ImapContextBuilder::new(
            account_config.clone(),
            imap_config.clone(),
        )),
        smtp: None,
    };
    let backend_builder = BackendBuilder::new(account_config.clone(), ctx_builder);
    let backend: BackendPool<MyContext> = backend_builder.build().await.unwrap();
    let folders = backend.list_folders().await.unwrap();

    assert_eq!(
        folders,
        Folders::from_iter([Folder {
            kind: Some(FolderKind::Inbox),
            name: "INBOX".into(),
            desc: "".into()
        }])
    );

    drop(backend);

    // TEST STATIC BACKEND

    // 1. define custom context made of subcontexts

    #[derive(BackendContextV2)]
    struct MyStaticContext {
        imap: ImapContextSync,
        smtp: SmtpContextSync,
    }

    // 2. implement context getters (proc-macro?)

    impl AsRef<ImapContextSync> for MyStaticContext {
        fn as_ref(&self) -> &ImapContextSync {
            &self.imap
        }
    }

    impl AsRef<SmtpContextSync> for MyStaticContext {
        fn as_ref(&self) -> &SmtpContextSync {
            &self.smtp
        }
    }

    // 3. define custom context builder made of subcontext builders

    #[derive(Clone)]
    struct MyStaticContextBuilder {
        imap: ImapContextBuilder,
        smtp: SmtpContextBuilder,
    }

    // 4. implement backend context builder

    #[async_trait]
    impl BackendContextBuilder for MyStaticContextBuilder {
        type Context = MyStaticContext;

        fn list_folders(&self) -> Option<BackendFeature<Self::Context, dyn ListFolders>> {
            self.list_folders_with(&self.imap)
        }

        async fn build(self) -> Result<Self::Context> {
            Ok(MyStaticContext {
                imap: self.imap.build().await?,
                smtp: self.smtp.build().await?,
            })
        }
    }

    // 5. define custom backend

    struct MyBackend(MyStaticContext);

    // 6. implement deref pointing to the context

    impl Deref for MyBackend {
        type Target = MyStaticContext;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    // 7. implement desired backend features

    #[async_trait]
    impl ListFolders for MyBackend {
        async fn list_folders(&self) -> Result<Folders> {
            ListImapFolders::new(&self.0.imap).list_folders().await
        }
    }

    // 8. plug all together

    let ctx_builder = MyStaticContextBuilder {
        imap: ImapContextBuilder::new(account_config.clone(), imap_config),
        smtp: SmtpContextBuilder::new(account_config, smtp_config),
    };

    let backend = MyBackend(ctx_builder.build().await.unwrap());

    assert!(backend.list_folders().await.is_ok());
}
