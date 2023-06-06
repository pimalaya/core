#[cfg(all(feature = "imap-backend", feature = "smtp-sender"))]
#[test]
fn test_smtp_sender() {
    use mail_builder::MessageBuilder;
    use pimalaya_email::{
        AccountConfig, Backend, ImapAuthConfig, ImapBackend, ImapConfig, PasswdConfig, Sender,
        Smtp, SmtpAuthConfig, SmtpConfig,
    };
    use pimalaya_secret::Secret;
    use std::{thread, time::Duration};

    env_logger::builder().is_test(true).init();

    let account_config = AccountConfig::default();
    let smtp_config = SmtpConfig {
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
    };

    let mut smtp = Smtp::new(&account_config, &smtp_config).unwrap();

    let imap = ImapBackend::new(
        account_config,
        ImapConfig {
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
        },
    )
    .unwrap();

    // setting up folders
    imap.purge_folder("INBOX").unwrap();

    // checking that an email can be built and sent
    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    smtp.send(&email).unwrap();

    thread::sleep(Duration::from_secs(1));

    // checking that the envelope of the sent email exists
    let envelopes = imap.list_envelopes("INBOX", 10, 0).unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);

    // clean up

    imap.purge_folder("INBOX").unwrap();
    imap.close().unwrap();
}
