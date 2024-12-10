use std::sync::Arc;

use async_trait::async_trait;
use email::{
    account::config::{passwd::PasswordConfig, AccountConfig},
    backend::{
        context::BackendContextBuilder, feature::BackendFeature, macros::BackendContext,
        mapper::SomeBackendContextBuilderMapper, Backend, BackendBuilder,
    },
    folder::{list::ListFolders, Folder, FolderKind},
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapContext, ImapContextBuilder,
    },
    smtp::{SmtpContextBuilder, SmtpContextSync},
    tls::Encryption,
    AnyResult,
};
use email_testing_server::with_email_testing_server;
use secret::Secret;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_dynamic_backend() {
    with_email_testing_server(|ports| async move {
        let account_config = Arc::new(AccountConfig::default());

        let imap_config = Arc::new(ImapConfig {
            host: "localhost".into(),
            port: ports.imap,
            encryption: Some(Encryption::None),
            login: "bob".into(),
            auth: ImapAuthConfig::Password(PasswordConfig(Secret::new_raw("password"))),
            ..Default::default()
        });

        // 1. define custom context

        #[derive(BackendContext)]
        struct DynamicContext {
            imap: Option<ImapContext>,
            smtp: Option<SmtpContextSync>,
        }

        // 2. implement AsRef for mapping features

        impl AsRef<Option<ImapContext>> for DynamicContext {
            fn as_ref(&self) -> &Option<ImapContext> {
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

            async fn build(self) -> AnyResult<Self::Context> {
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
        let backend: Backend<DynamicContext> = backend_builder.build().await.unwrap();
        let folders = backend.list_folders().await.unwrap();

        assert!(folders.contains(&Folder {
            kind: Some(FolderKind::Inbox),
            name: "INBOX".into(),
            desc: "".into()
        }));
    })
    .await
}
