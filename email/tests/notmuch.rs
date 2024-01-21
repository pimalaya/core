use concat_with::concat_line;
use email::{
    account::config::AccountConfig,
    backend::BackendBuilder,
    envelope::{list::notmuch::ListNotmuchEnvelopes, Id},
    flag::{
        add::notmuch::AddNotmuchFlags, remove::notmuch::RemoveNotmuchFlags,
        set::notmuch::SetNotmuchFlags, Flag, Flags,
    },
    folder::{config::FolderConfig, INBOX},
    message::{add::notmuch::AddNotmuchMessage, peek::notmuch::PeekNotmuchMessages},
    notmuch::{config::NotmuchConfig, NotmuchContextBuilder},
};
use mail_builder::MessageBuilder;
use maildirpp::Maildir;
use notmuch::Database;
use std::{collections::HashMap, fs, iter::FromIterator};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread")]
async fn test_notmuch_features() {
    env_logger::builder().is_test(true).init();

    // set up maildir folders and notmuch database

    let mdir: Maildir = tempdir().unwrap().path().to_owned().into();
    if let Err(_) = fs::remove_dir_all(mdir.path()) {}
    mdir.create_dirs().unwrap();

    let custom_mdir: Maildir = mdir.path().join("CustomMaildirFolder").into();
    if let Err(_) = fs::remove_dir_all(custom_mdir.path()) {}
    custom_mdir.create_dirs().unwrap();

    Database::create(mdir.path()).unwrap();

    let account_config = AccountConfig {
        name: "account".into(),
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([
                (INBOX.into(), "folder:\"\"".into()),
                ("custom".into(), "folder:\"CustomMaildirFolder\"".into()),
            ])),
            ..Default::default()
        }),
        ..Default::default()
    };

    let notmuch_config = NotmuchConfig {
        database_path: mdir.path().to_owned(),
        ..Default::default()
    };

    let notmuch_ctx = NotmuchContextBuilder::new(account_config.clone(), notmuch_config);
    let notmuch = BackendBuilder::new(account_config.clone(), notmuch_ctx)
        .with_list_envelopes(|ctx| Some(ListNotmuchEnvelopes::new_boxed(ctx.clone())))
        .with_add_flags(|ctx| Some(AddNotmuchFlags::new_boxed(ctx.clone())))
        .with_set_flags(|ctx| Some(SetNotmuchFlags::new_boxed(ctx.clone())))
        .with_remove_flags(|ctx| Some(RemoveNotmuchFlags::new_boxed(ctx.clone())))
        .with_add_message(|ctx| Some(AddNotmuchMessage::new_boxed(ctx.clone())))
        .with_peek_messages(|ctx| Some(PeekNotmuchMessages::new_boxed(ctx.clone())))
        .build()
        .await
        .unwrap();

    // check that messages can be added

    let msg = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message custom!")
        .text_body("Plain message custom!")
        .write_to_vec()
        .unwrap();
    notmuch
        .add_message_with_flag("custom", &msg, Flag::Seen)
        .await
        .unwrap();

    let flags = Flags::from_iter([Flag::custom("flag"), Flag::Seen]);
    let msg = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();

    let id = notmuch
        .add_message_with_flags(INBOX, &msg, &flags)
        .await
        .unwrap();

    // check that the added message exists

    let msgs = notmuch.get_messages(INBOX, &Id::single(id)).await.unwrap();

    let tpl = msgs
        .to_vec()
        .first()
        .unwrap()
        .to_read_tpl(&account_config, |i| {
            i.with_show_only_headers(["From", "To"])
        })
        .await
        .unwrap();

    let expected_tpl = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "",
        "Plain message!",
        "",
    );

    assert_eq!(tpl, expected_tpl);

    // check that the envelope of the added message exists

    let envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message custom!", envelope.subject);

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);

    // check that a flag can be added to the envelope

    let flags = Flags::from_iter([Flag::Flagged, Flag::Answered]);
    notmuch
        .add_flags(INBOX, &Id::single(&envelope.id), &flags)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that the envelope flags can be changed

    let flags = Flags::from_iter([Flag::custom("flag"), Flag::Answered]);
    notmuch
        .set_flags("", &Id::single(&envelope.id), &flags)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from the envelope

    notmuch
        .remove_flag("", &Id::single(&envelope.id), Flag::Answered)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be deleted
    // notmuch.delete_emails("", vec![&id]).await.unwrap();
    // assert!(notmuch.get_emails(INBOX, vec![&id]).await.is_err());
}
