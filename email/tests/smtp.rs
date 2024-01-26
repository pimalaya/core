use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    folder::INBOX,
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
    smtp::{
        config::{SmtpAuthConfig, SmtpConfig, SmtpEncryptionKind},
        SmtpContextBuilder,
    },
};
use mail_builder::MessageBuilder;
use secret::Secret;
use std::{sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread")]
async fn test_smtp_features() {
    env_logger::builder().is_test(true).init();

    let account_config = Arc::new(AccountConfig::default());

    let imap_config = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: 3143,
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("echo 'password'"))),
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

    let imap_ctx = ImapContextBuilder::new(imap_config);
    let imap = BackendBuilder::new(account_config.clone(), imap_ctx)
        .build()
        .await
        .unwrap();

    let smtp_ctx = SmtpContextBuilder::new(smtp_config);
    let smtp = BackendBuilder::new(account_config.clone(), smtp_ctx)
        .build()
        .await
        .unwrap();

    // setting up folders

    imap.purge_folder(INBOX).await.unwrap();

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

    let envelopes = imap.list_envelopes("INBOX", 10, 0).await.unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);
}
