#[cfg(feature = "imap-backend")]
#[test]
fn test_sendmail_sender() {
    use mail_builder::MessageBuilder;
    use pimalaya_email::{
        AccountConfig, Backend, ImapAuthConfig, ImapBackend, ImapConfig, PasswdConfig, Sender,
        Sendmail, SendmailConfig,
    };
    use pimalaya_secret::Secret;
    use std::{thread, time::Duration};

    env_logger::builder().is_test(true).init();

    let account_config = AccountConfig::default();
    let sendmail_config = SendmailConfig {
        cmd: [
            "msmtp",
            "--host localhost",
            "--port 3025",
            "--user=alice@localhost",
            "--passwordeval='echo password'",
            "--read-envelope-from",
            "--read-recipients",
        ]
        .join(" ")
        .into(),
    };
    let mut sendmail = Sendmail::new(&account_config, &sendmail_config);
    let imap = ImapBackend::new(
        account_config.clone(),
        ImapConfig {
            host: "localhost".into(),
            port: 3143,
            ssl: Some(false),
            login: "bob@localhost".into(),
            auth: ImapAuthConfig::Passwd(PasswdConfig {
                passwd: Secret::new_raw("password"),
            }),
            ..ImapConfig::default()
        },
    )
    .unwrap();

    // setting up folders
    imap.purge_folder("INBOX").unwrap();

    // checking that an email can be sent
    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    sendmail.send(&email).unwrap();

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
