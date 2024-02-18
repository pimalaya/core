use async_trait::async_trait;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend_v2::{
        context::BackendContextBuilder, feature::BackendFeature, macros::BackendContextV2,
        mapper::SomeBackendContextBuilderMapper, pool::BackendPool, BackendBuilder,
    },
    folder::{list::ListFolders, Folder, FolderKind, Folders},
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder, ImapContextSync,
    },
    smtp::{SmtpContextBuilder, SmtpContextSync},
    Result,
};
use secret::Secret;
use std::sync::Arc;

#[tokio::test]
async fn test_dynamic_backend() {
    env_logger::builder().is_test(true).init();

    let account_config = Arc::new(AccountConfig::default());

    let imap_config = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: 3143,
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..Default::default()
    });

    // 1. define custom context

    #[derive(BackendContextV2)]
    struct DynamicContext {
        imap: Option<ImapContextSync>,
        smtp: Option<SmtpContextSync>,
    }

    // 2. implement AsRef for mapping features

    impl AsRef<Option<ImapContextSync>> for DynamicContext {
        fn as_ref(&self) -> &Option<ImapContextSync> {
            &self.imap
        }
    }

    impl AsRef<Option<SmtpContextSync>> for DynamicContext {
        fn as_ref(&self) -> &Option<SmtpContextSync> {
            &self.smtp
        }
    }

    // 3. define custom context builder

    #[derive(Clone)]
    struct DynamicContextBuilder {
        imap: Option<ImapContextBuilder>,
        smtp: Option<SmtpContextBuilder>,
    }

    // 4. implement backend context builder

    #[async_trait]
    impl BackendContextBuilder for DynamicContextBuilder {
        type Context = DynamicContext;

        // override the list folders feature using the imap builder
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

            Ok(DynamicContext { imap, smtp })
        }
    }

    // 5. plug all together

    let ctx_builder = DynamicContextBuilder {
        imap: Some(ImapContextBuilder::new(
            account_config.clone(),
            imap_config.clone(),
        )),
        smtp: None,
    };
    let backend_builder = BackendBuilder::new(account_config.clone(), ctx_builder);
    let backend: BackendPool<DynamicContext> = backend_builder.build().await.unwrap();
    let folders = backend.list_folders().await.unwrap();

    assert!(folders.contains(&Folder {
        kind: Some(FolderKind::Inbox),
        name: "INBOX".into(),
        desc: "".into()
    }));
}
