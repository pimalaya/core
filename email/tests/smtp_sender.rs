#[tokio::test(flavor = "multi_thread")]
async fn smtp_sender() {
    use mail_builder::MessageBuilder;
    use pimalaya_email::{
        account::{AccountConfig, PasswdConfig},
        backend::{BackendBuilder, BackendConfig, ImapAuthConfig, ImapConfig},
        sender::{SenderBuilder, SenderConfig, SmtpAuthConfig, SmtpConfig},
    };
    use pimalaya_secret::Secret;
    use std::time::Duration;

    env_logger::builder().is_test(true).init();

    let config = AccountConfig {
        backend: BackendConfig::Imap(ImapConfig {
            host: "localhost".into(),
            port: 3143,
            ssl: Some(false),
            starttls: Some(false),
            insecure: Some(true),
            login: "bob@localhost".into(),
            auth: ImapAuthConfig::Passwd(PasswdConfig {
                passwd: Secret::new_raw("echo 'password'"),
            }),
            ..ImapConfig::default()
        }),
        sender: SenderConfig::Smtp(SmtpConfig {
            host: "localhost".into(),
            port: 3025,
            ssl: Some(false),
            starttls: Some(false),
            insecure: Some(true),
            login: "alice@localhost".into(),
            auth: SmtpAuthConfig::Passwd(PasswdConfig {
                passwd: Secret::new_raw("password"),
            }),
            ..SmtpConfig::default()
        }),
        ..AccountConfig::default()
    };

    let imap_builder = BackendBuilder::new(config.clone());
    let mut imap = imap_builder.build().await.unwrap();

    let smtp_builder = SenderBuilder::new(config.clone());
    let mut smtp = smtp_builder.build().await.unwrap();

    // setting up folders

    imap.purge_folder("INBOX").await.unwrap();

    // checking that an email can be built and sent

    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    smtp.send(&email).await.unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    // checking that the envelope of the sent email exists

    let envelopes = imap.list_envelopes("INBOX", 10, 0).await.unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);
}
