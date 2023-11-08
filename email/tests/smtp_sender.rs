#[cfg(feature = "imap-backend")]
#[cfg(feature = "smtp-sender")]
#[tokio::test(flavor = "multi_thread")]
async fn test_smtp_features() {
    use email::{
        account::{AccountConfig, PasswdConfig},
        backend::{BackendBuilderV2, BackendConfig, ImapAuthConfig, ImapConfig},
        email::{
            envelope::list::imap::ListImapEnvelopes, message::send_raw::smtp::SendRawMessageSmtp,
        },
        folder::purge::imap::PurgeImapFolder,
        imap::ImapSessionBuilder,
        prelude::*,
        sender::{SenderConfig, SmtpAuthConfig, SmtpConfig},
        smtp::SmtpClientBuilder,
    };
    use mail_builder::MessageBuilder;
    use secret::Secret;
    use std::time::Duration;

    env_logger::builder().is_test(true).init();

    let imap_config = ImapConfig {
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
    };
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
    let config = AccountConfig {
        backend: BackendConfig::Imap(imap_config.clone()),
        sender: SenderConfig::Smtp(smtp_config.clone()),
        ..AccountConfig::default()
    };

    let imap_ctx = ImapSessionBuilder::new(config.clone(), imap_config);
    let smtp_ctx = SmtpClientBuilder::new(config.clone(), smtp_config);
    let backend_builder = BackendBuilderV2::new((imap_ctx, smtp_ctx))
        .with_purge_folder(|ctx| PurgeImapFolder::new(&ctx.0))
        .with_list_envelopes(|ctx| ListImapEnvelopes::new(&ctx.0))
        .with_send_raw_message(|ctx| SendRawMessageSmtp::new(&ctx.1));
    let backend = backend_builder.build().await.unwrap();

    // setting up folders

    backend.purge_folder("INBOX").await.unwrap();

    // checking that an email can be built and sent

    let raw_msg = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    backend.send_raw_message(&raw_msg).await.unwrap();

    tokio::time::sleep(Duration::from_secs(1)).await;

    // checking that the envelope of the sent email exists

    let envelopes = backend.list_envelopes("INBOX", 10, 0).await.unwrap();
    assert_eq!(1, envelopes.len());
    let envelope = envelopes.first().unwrap();
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);
}
