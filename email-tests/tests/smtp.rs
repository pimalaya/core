use std::{sync::Arc, time::Duration};

use email::{
    account::config::{passwd::PasswordConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::list::ListEnvelopes,
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapContextBuilder,
    },
    message::send::SendMessage,
    smtp::{
        config::{SmtpAuthConfig, SmtpConfig},
        SmtpContextBuilder,
    },
    tls::Encryption,
};
use email_testing_server::with_email_testing_server;
use mail_builder::MessageBuilder;
use secret::Secret;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_smtp_features() {
    with_email_testing_server(|ports| async move {
        let account_config = Arc::new(AccountConfig::default());

        let imap_config = Arc::new(ImapConfig {
            host: "localhost".into(),
            port: ports.imap,
            encryption: Some(Encryption::None),
            login: "bob".into(),
            auth: ImapAuthConfig::Password(PasswordConfig(Secret::new_command("echo 'password'"))),
            ..Default::default()
        });

        let smtp_config = Arc::new(SmtpConfig {
            host: "localhost".into(),
            port: ports.smtp,
            encryption: Some(Encryption::None),
            login: "alice".into(),
            auth: SmtpAuthConfig::Password(PasswordConfig(Secret::new_raw("password"))),
        });

        let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config);
        let imap = BackendBuilder::new(account_config.clone(), imap_ctx)
            .build()
            .await
            .unwrap();

        let smtp_ctx = SmtpContextBuilder::new(account_config.clone(), smtp_config);
        let smtp = BackendBuilder::new(account_config.clone(), smtp_ctx)
            .build()
            .await
            .unwrap();

        // checking that an email can be built and sent

        let raw_msg = MessageBuilder::new()
            .from("alice@localhost")
            .to("bob@localhost")
            .subject("Plain message!")
            .text_body("Plain message!")
            .write_to_vec()
            .unwrap();
        smtp.send_message(&raw_msg).await.unwrap();

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
