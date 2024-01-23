use async_trait::async_trait;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::{
        BackendBuilderV2, BackendContextBuilder, BackendContextBuilderV2, BackendContextMapper,
        BackendFeature, BackendFeatureMapper,
    },
    folder::{
        config::FolderConfig,
        list::{imap::ListImapFolders, ListFolders},
        Folders, SENT,
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
use std::collections::HashMap;

#[tokio::test]
async fn test_backend_v2() {
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

    #[derive(Clone)]
    struct MyContextBuilder {
        imap: ImapContextBuilder,
        smtp: SmtpContextBuilder,
    }

    struct MyContext {
        imap: ImapContextSync,
        smtp: SmtpContextSync,
    }

    // TEST DYNAMIC BACKEND

    // 1. implement context mappers (can be easily auto-implemented by macros)

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

    // 2. implement feature mapper (can be easily auto-implemented by macros)

    impl<C1: BackendContextMapper<C2>, C2: Send + 'static> BackendFeatureMapper<C1, C2>
        for MyContextBuilder
    {
    }

    // 3. implement backend context builder

    #[async_trait]
    impl BackendContextBuilderV2 for MyContextBuilder {
        type Context = MyContext;

        #[cfg(feature = "folder-list")]
        fn list_folders(&self) -> BackendFeature<Self::Context, dyn ListFolders> {
            self.map_feature(self.imap.list_folders())
        }

        async fn build(self, account_config: &AccountConfig) -> Result<Self::Context> {
            Ok(MyContext {
                imap: BackendContextBuilderV2::build(self.imap, account_config).await?,
                smtp: self.smtp.build(account_config).await?,
            })
        }
    }

    let ctx_builder = MyContextBuilder {
        imap: ImapContextBuilder::new(imap_config.clone()),
        smtp: SmtpContextBuilder::new(smtp_config.clone()),
    };

    let backend_v2 = BackendBuilderV2::new(ctx_builder)
        .build(account_config.clone())
        .await
        .unwrap();

    assert!(backend_v2.list_folders().await.is_ok());

    // TEST STATIC BACKEND

    pub struct MyBackend(MyContext);

    #[async_trait]
    impl ListFolders for MyBackend {
        async fn list_folders(&self) -> Result<Folders> {
            ListImapFolders::new(&self.0.imap).list_folders().await
        }
    }

    let ctx_builder = MyContextBuilder {
        imap: ImapContextBuilder::new(imap_config.clone()),
        smtp: SmtpContextBuilder::new(smtp_config.clone()),
    };

    let backend = MyBackend(ctx_builder.build(&account_config).await.unwrap());

    assert!(backend.list_folders().await.is_ok());
}
