#![cfg(feature = "full")]

use std::sync::Arc;

use async_trait::async_trait;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::{context::BackendContextBuilder, macros::BackendContext},
    folder::{
        list::{imap::ListImapFolders, ListFolders},
        Folder, FolderKind, Folders,
    },
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContext, ImapContextBuilder,
    },
    message::send::{smtp::SendSmtpMessage, SendMessage},
    smtp::{
        config::{SmtpAuthConfig, SmtpConfig, SmtpEncryptionKind},
        SmtpContextBuilder, SmtpContextSync,
    },
    AnyResult,
};
use email_testing_server::with_email_testing_server;
use secret::Secret;

#[tokio::test(flavor = "multi_thread")]
async fn test_static_backend() {
    env_logger::builder().is_test(true).init();

    with_email_testing_server(|ports| async move {
        let account_config = Arc::new(AccountConfig::default());

        let imap_config = Arc::new(ImapConfig {
            host: "localhost".into(),
            port: ports.imap,
            encryption: Some(ImapEncryptionKind::None),
            login: "bob".into(),
            auth: ImapAuthConfig::Password(PasswdConfig(Secret::new_raw("password"))),
            ..Default::default()
        });

        let smtp_config = Arc::new(SmtpConfig {
            host: "localhost".into(),
            port: ports.smtp,
            encryption: Some(SmtpEncryptionKind::None),
            login: "alice".into(),
            auth: SmtpAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        });

        // 1. define custom context made of subcontexts

        #[derive(BackendContext)]
        struct StaticContext {
            imap: ImapContext,
            smtp: SmtpContextSync,
        }

        // 2. define custom backend

        struct StaticBackend(StaticContext);

        // 3. implement desired backend features

        #[async_trait]
        impl ListFolders for StaticBackend {
            async fn list_folders(&self) -> AnyResult<Folders> {
                ListImapFolders::new(&self.0.imap).list_folders().await
            }
        }

        #[async_trait]
        impl SendMessage for StaticBackend {
            async fn send_message(&self, msg: &[u8]) -> AnyResult<()> {
                SendSmtpMessage::new(&self.0.smtp).send_message(msg).await
            }
        }

        // 4. plug all together

        let backend = StaticBackend(StaticContext {
            imap: ImapContextBuilder::new(account_config.clone(), imap_config)
                .build()
                .await
                .unwrap(),
            smtp: SmtpContextBuilder::new(account_config, smtp_config)
                .build()
                .await
                .unwrap(),
        });

        let folders = backend.list_folders().await.unwrap();

        assert!(folders.contains(&Folder {
            kind: Some(FolderKind::Inbox),
            name: "INBOX".into(),
            desc: "".into()
        }));
    })
    .await
}
