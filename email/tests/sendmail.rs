use email::{
    account::{AccountConfig, PasswdConfig},
    backend::BackendBuilder,
    email::{
        envelope::list::imap::ListEnvelopesImap,
        message::send_raw::sendmail::SendRawMessageSendmail,
    },
    folder::purge::imap::PurgeFolderImap,
    imap::{ImapAuthConfig, ImapConfig, ImapSessionBuilder},
    sendmail::{SendmailConfig, SendmailContext},
};
use mail_builder::MessageBuilder;
use secret::Secret;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test_sendmail_features() {
    env_logger::builder().is_test(true).init();

    let account_config = AccountConfig::default();
    let imap_config = ImapConfig {
        host: "localhost".into(),
        port: 3143,
        ssl: Some(false),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig {
            passwd: Secret::new_raw("password"),
        }),
        ..Default::default()
    };
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

    let imap_ctx = ImapSessionBuilder::new(account_config.clone(), imap_config);
    let imap_builder = BackendBuilder::new(account_config.clone(), imap_ctx)
        .with_purge_folder(PurgeFolderImap::new)
        .with_list_envelopes(ListEnvelopesImap::new);
    let imap = imap_builder.build().await.unwrap();

    let sendmail_ctx = SendmailContext::new(account_config.clone(), sendmail_config);
    let sendmail_builder = BackendBuilder::new(account_config.clone(), sendmail_ctx)
        .with_send_raw_message(SendRawMessageSendmail::new);
    let sendmail = sendmail_builder.build().await.unwrap();

    // setting up folders

    imap.purge_folder("INBOX").await.unwrap();

    // checking that an email can be sent

    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    sendmail.send_raw_message(&email).await.unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    // checking that the envelope of the sent email exists

    let envelopes = imap.list_envelopes("INBOX", 10, 0).await.unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);
}
