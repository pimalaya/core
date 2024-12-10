use std::{iter::FromIterator, sync::Arc};

use email::{
    account::config::{passwd::PasswordConfig, AccountConfig},
    backend::BackendBuilder,
    envelope::{
        list::{ListEnvelopes, ListEnvelopesOptions},
        Envelope, Envelopes,
    },
    flag::{Flag, Flags},
    folder::INBOX,
    imap::{
        config::{ImapAuthConfig, ImapConfig},
        ImapContextBuilder,
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    message::add::AddMessage,
    notmuch::{config::NotmuchConfig, NotmuchContextBuilder},
    sync::SyncBuilder,
    tls::Encryption,
};
use email_testing_server::start_email_testing_server;
use mail_builder::MessageBuilder;
use secret::Secret;
use tempfile::tempdir;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_search_emails_query() {
    let (ports, shutdown) = start_email_testing_server().await;

    let tmp = tempdir().unwrap();
    let tmp = tmp.path();

    let account_config = Arc::new(AccountConfig {
        name: "test".into(),
        ..Default::default()
    });

    // set up IMAP

    let imap_config = Arc::new(ImapConfig {
        host: "localhost".into(),
        port: ports.imap,
        encryption: Some(Encryption::None),
        login: "alice".into(),
        auth: ImapAuthConfig::Password(PasswordConfig(Secret::new_raw("password"))),
        ..Default::default()
    });

    let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config.clone());
    let imap_builder = BackendBuilder::new(account_config.clone(), imap_ctx);
    let imap = imap_builder.clone().build().await.unwrap();

    imap.add_message_with_flag(
        INBOX,
        &MessageBuilder::new()
            // January, 2024 the 1st at 12:00 (UTC)
            .date(1704106800_i64)
            .message_id("a@localhost")
            .from("bob@localhost")
            .to("alice@localhost")
            .subject("A")
            .text_body("A")
            .write_to_vec()
            .unwrap(),
        Flag::Seen,
    )
    .await
    .unwrap();

    imap.add_message_with_flags(
        INBOX,
        &MessageBuilder::new()
            // January, 2024 the 5th at 12:00 (UTC)
            .date(1704452400_i64)
            .message_id("b@localhost")
            .from("claire@localhost")
            .to("alice@localhost")
            .subject("B")
            .text_body("B")
            .write_to_vec()
            .unwrap(),
        &Flags::from_iter([Flag::Seen, Flag::Flagged, Flag::Custom("custom".into())]),
    )
    .await
    .unwrap();

    imap.add_message(
        INBOX,
        &MessageBuilder::new()
            // January, 2024 the 10th at 12:00 (UTC)
            .date(1704884400_i64)
            .message_id("c@localhost")
            .from(("Dminic", "dominic@localhost"))
            .to("bob@localhost")
            .subject("C")
            .text_body("C")
            .write_to_vec()
            .unwrap(),
    )
    .await
    .unwrap();

    drop(imap);

    // set up Maildir

    let mdir_config = Arc::new(MaildirConfig {
        root_dir: tmp.join("maildir"),
        maildirpp: false,
    });

    let mdir_ctx = MaildirContextBuilder::new(account_config.clone(), mdir_config.clone());
    let mdir_builder = BackendBuilder::new(account_config.clone(), mdir_ctx);
    let mdir = mdir_builder.clone().build().await.unwrap();

    // set up Notmuch

    let notmuch_db_path = mdir_config.root_dir.clone();
    notmuch::Database::create(&notmuch_db_path).unwrap();

    let notmuch_config = Arc::new(NotmuchConfig {
        database_path: Some(notmuch_db_path),
        maildir_path: Some(mdir_config.root_dir.clone()),
        ..Default::default()
    });

    let notmuch_ctx = NotmuchContextBuilder::new(account_config.clone(), notmuch_config.clone());
    let notmuch_builder = BackendBuilder::new(account_config.clone(), notmuch_ctx);
    let notmuch = notmuch_builder.clone().build().await.unwrap();

    // sync IMAP with Notmuch

    SyncBuilder::new(notmuch_builder.clone(), imap_builder.clone())
        .with_cache_dir(tmp.join("sync-cache"))
        .sync()
        .await
        .unwrap();

    // test query

    let imap = imap_builder.build().await.unwrap();

    let query = "order by date";
    let expected_msg_ids = ["a", "b", "c"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "order by date desc";
    let expected_msg_ids = ["c", "b", "a"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "date 05/01/2024";
    let expected_msg_ids = ["b"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "not date 05/01/2024 order by subject";
    let expected_msg_ids = ["a", "c"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "before 05/01/2024";
    let expected_msg_ids = ["a"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "after 05/01/2024";
    let expected_msg_ids = ["c"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "subject a and body a or body b order by subject asc";
    let expected_msg_ids = ["a", "b"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "subject a and (body a or body b) order by subject";
    let expected_msg_ids = ["a"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "from Dminic or from bob order by subject";
    let expected_msg_ids = ["a", "c"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "not to bob order by subject";
    let expected_msg_ids = ["a", "b"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    let query = "flag seen order by subject desc";
    let expected_msg_ids = ["b", "a"];

    let (got, expected) = test_query(&imap, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&mdir, query, expected_msg_ids).await;
    assert_eq!(got, expected);
    let (got, expected) = test_query(&notmuch, query, expected_msg_ids).await;
    assert_eq!(got, expected);

    shutdown()
}

async fn test_query(
    backend: &impl ListEnvelopes,
    query: &str,
    msg_ids: impl IntoIterator<Item = &str>,
) -> (Envelopes, Envelopes) {
    let query = query.parse().unwrap();
    let envelopes = backend
        .list_envelopes(
            INBOX,
            ListEnvelopesOptions {
                page_size: 0,
                page: 0,
                query: Some(query),
            },
        )
        .await
        .unwrap();

    let expected_envelopes = Envelopes::from_iter(msg_ids.into_iter().map(|msg_id| Envelope {
        message_id: format!("<{msg_id}@localhost>"),
        ..Default::default()
    }));

    (envelopes, expected_envelopes)
}
