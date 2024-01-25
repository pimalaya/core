use async_trait::async_trait;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::{
        macros::BackendContext, BackendBuilder, BackendContextBuilder, BackendFeatureBuilder,
        FindBackendSubcontext, GetBackendSubcontext, MapBackendFeature,
    },
    folder::{config::FolderConfig, list::ListFolders, SENT},
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

    #[derive(BackendContext)]
    struct MyContext {
        imap: Option<ImapContextSync>,
        smtp: Option<SmtpContextSync>,
    }

    // 2. implement subcontexts (could be auto-implemented by macros)

    impl FindBackendSubcontext<ImapContextSync> for MyContext {
        fn find_subcontext(&self) -> Option<&ImapContextSync> {
            self.imap.as_ref()
        }
    }

    impl FindBackendSubcontext<SmtpContextSync> for MyContext {
        fn find_subcontext(&self) -> Option<&SmtpContextSync> {
            self.smtp.as_ref()
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

        #[cfg(feature = "folder-list")]
        fn list_folders(
            &self,
        ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn ListFolders>>> {
            self.list_folders_from(self.imap.as_ref())
        }

        async fn build(self, account_config: Arc<AccountConfig>) -> Result<Self::Context> {
            let imap = match self.imap {
                Some(imap) => Some(imap.build(account_config.clone()).await?),
                None => None,
            };

            let smtp = match self.smtp {
                Some(smtp) => Some(smtp.build(account_config).await?),
                None => None,
            };

            Ok(MyContext { imap, smtp })
        }
    }

    // 5. plug all together

    let ctx_builder = MyContextBuilder {
        imap: Some(ImapContextBuilder::new(imap_config.clone())),
        smtp: None,
    };
    let backend_builder = BackendBuilder::new(account_config.clone(), ctx_builder);
    let backend = backend_builder.build().await.unwrap();

    assert!(backend.list_folders().await.is_ok());

    // TEST STATIC BACKEND

    // 1. define custom context made of subcontexts

    #[derive(BackendContext)]
    struct MyStaticContext {
        imap: ImapContextSync,
        smtp: SmtpContextSync,
    }

    // 2. implement context getters (proc-macro?)

    impl GetBackendSubcontext<ImapContextSync> for MyStaticContext {
        fn get_subcontext(&self) -> &ImapContextSync {
            &self.imap
        }
    }

    impl GetBackendSubcontext<SmtpContextSync> for MyStaticContext {
        fn get_subcontext(&self) -> &SmtpContextSync {
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

        #[cfg(feature = "folder-list")]
        fn list_folders(
            &self,
        ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn ListFolders>>> {
            self.list_folders_from(Some(&self.imap))
        }

        async fn build(self, account_config: Arc<AccountConfig>) -> Result<Self::Context> {
            Ok(MyStaticContext {
                imap: self.imap.build(account_config.clone()).await?,
                smtp: self.smtp.build(account_config).await?,
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

    // 8. plug all together

    let ctx_builder = MyStaticContextBuilder {
        imap: ImapContextBuilder::new(imap_config),
        smtp: SmtpContextBuilder::new(smtp_config),
    };

    let backend = MyBackend(ctx_builder.build(account_config).await.unwrap());

    assert!(backend.list_folders().await.is_ok());
}
