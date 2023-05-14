#[cfg(feature = "imap-backend")]
use std::{thread, time::Duration};

use pimalaya_email::{
    AccountConfig, CompilerBuilder, Sender, Sendmail, SendmailConfig, TplBuilder,
};

#[cfg(feature = "imap-backend")]
use pimalaya_email::{Backend, ImapBackend, ImapConfig};

#[cfg(feature = "imap-backend")]
#[test]
fn test_sendmail_sender() {
    use pimalaya_email::{ImapAuthConfig, PasswdConfig};
    use pimalaya_secret::Secret;

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
        .join(" "),
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
    let email = TplBuilder::default()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_plain_part("Plain message!")
        .compile(CompilerBuilder::default())
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
