use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::BackendBuilder,
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder,
    },
    sendmail::{config::SendmailConfig, SendmailContextBuilder},
};
use mail_builder::MessageBuilder;
use secret::Secret;
use std::{sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread")]
async fn test_sendmail_features() {
    env_logger::builder().is_test(true).init();

    let account_config = Arc::new(AccountConfig::default());

    let imap_config = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: 3143,
        encryption: Some(ImapEncryptionKind::None),
        login: "bob@localhost".into(),
        auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
        ..Default::default()
    });

    let sendmail_config = Arc::new(SendmailConfig {
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

    let envelopes = imap.list_envelopes("INBOX", 10, 0).await.unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);
}
