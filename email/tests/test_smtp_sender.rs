use std::{borrow::Cow, thread, time::Duration};

use himalaya_lib::{AccountConfig, CompilerBuilder, Sender, TplBuilder};

#[cfg(feature = "imap-backend")]
use himalaya_lib::{Backend, ImapBackend, ImapConfig};
#[cfg(feature = "smtp-sender")]
use himalaya_lib::{Smtp, SmtpConfig};

#[cfg(all(feature = "imap-backend", feature = "smtp-sender"))]
#[test]
fn test_smtp_sender() {
    env_logger::builder().is_test(true).init();

    let account_config = AccountConfig::default();

    let smtp_config = SmtpConfig {
        host: "localhost".into(),
        port: 3025,
        ssl: Some(false),
        starttls: Some(false),
        insecure: Some(true),
        login: "alice@localhost".into(),
        passwd_cmd: "echo 'password'".into(),
        ..SmtpConfig::default()
    };
    let mut smtp = Smtp::new(&account_config, &smtp_config);

    let imap = ImapBackend::new(
        Cow::Borrowed(&account_config),
        Cow::Owned(ImapConfig {
            host: "localhost".into(),
            port: 3143,
            ssl: Some(false),
            starttls: Some(false),
            insecure: Some(true),
            login: "bob@localhost".into(),
            passwd_cmd: "echo password".into(),
            ..ImapConfig::default()
        }),
    )
    .unwrap();

    // setting up folders
    imap.purge_folder("INBOX").unwrap();

    // checking that an email can be built and sent
    let email = TplBuilder::default()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_plain_part("Plain message!")
        .compile(CompilerBuilder::default())
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
