use std::{sync::Arc, time::Duration};

use email::{
    account::config::{passwd::PasswordConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::list::ListEnvelopes,
    folder::{delete::DeleteFolder, list::ListFolders, purge::PurgeFolder},
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapContextBuilder,
    },
    message::send::SendMessage,
    sendmail::{config::SendmailConfig, SendmailContextBuilder},
    tls::Encryption,
};
use email_testing_server::with_email_testing_server;
use mail_builder::MessageBuilder;
use process::Command;
use secret::Secret;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_sendmail_features() {
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

        let sendmail_config = Arc::new(SendmailConfig {
            cmd: Some(Command::new(
                [
                    "sendmail",
                    "--host localhost",
                    &format!("--port {}", ports.smtp),
                    "--user=alice@localhost",
                    "--passwordeval='echo password'",
                    "--read-envelope-from",
                    "--read-recipients",
                ]
                .join(" "),
            )),
        });

        let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config);
        let imap = BackendBuilder::new(account_config.clone(), imap_ctx)
            .build()
            .await
            .unwrap();

        let sendmail_ctx = SendmailContextBuilder::new(account_config.clone(), sendmail_config);
        let sendmail = BackendBuilder::new(account_config, sendmail_ctx)
            .build()
            .await
            .unwrap();

        // setting up folders

        for folder in imap.list_folders().await.unwrap().iter() {
            let _ = imap.purge_folder(&folder.name).await;
            let _ = imap.delete_folder(&folder.name).await;
        }

        // checking that an email can be sent

        let email = MessageBuilder::new()
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("Plain message!")
            .text_body("Plain message!")
            .write_to_vec()
            .unwrap();
        sendmail.send_message(&email).await.unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;

        // checking that the envelope of the sent email exists

        let envelopes = imap
            .list_envelopes("INBOX", Default::default())
            .await
            .unwrap();
        assert_eq!(1, envelopes.len());
        let envelope = envelopes.first().unwrap();
        assert_eq!("alice@localhost", envelope.from.addr);
        assert_eq!("Plain message!", envelope.subject);
    })
    .await
}
