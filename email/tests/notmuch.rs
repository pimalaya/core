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
    message::{
        add::notmuch::AddNotmuchMessage, copy::notmuch::CopyNotmuchMessages,
        move_::notmuch::MoveNotmuchMessages, peek::notmuch::PeekNotmuchMessages,
    },
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
            aliases: Some(HashMap::from_iter([(
                "custom".into(),
                "CustomMaildirFolder".into(),
            )])),
            ..Default::default()
        }),
        ..Default::default()
    };

    let notmuch_config = NotmuchConfig {
        database_path: Some(mdir.path().to_owned()),
        ..Default::default()
    };

    let notmuch_ctx = NotmuchContextBuilder::new(notmuch_config);
    let notmuch = BackendBuilder::new(account_config.clone(), notmuch_ctx)
        .with_list_envelopes(ListNotmuchEnvelopes::some_new_boxed)
        .with_add_flags(AddNotmuchFlags::some_new_boxed)
        .with_set_flags(SetNotmuchFlags::some_new_boxed)
        .with_remove_flags(RemoveNotmuchFlags::some_new_boxed)
        .with_add_message(AddNotmuchMessage::some_new_boxed)
        .with_peek_messages(PeekNotmuchMessages::some_new_boxed)
        .with_copy_messages(CopyNotmuchMessages::some_new_boxed)
        .with_move_messages(MoveNotmuchMessages::some_new_boxed)
        .build()
        .await
        .unwrap();

    // check that messages can be added

    let inbox_flags = Flags::from_iter([Flag::custom("flag"), Flag::Seen]);
    let inbox_msg = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    let inbox_id = notmuch
        .add_message_with_flags(INBOX, &inbox_msg, &inbox_flags)
        .await
        .unwrap();

    let custom_msg = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message custom!")
        .text_body("Plain message custom!")
        .write_to_vec()
        .unwrap();
    let custom_id = notmuch
        .add_message_with_flag("custom", &custom_msg, Flag::Seen)
        .await
        .unwrap();

    // check that the envelope of the added message exists

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let inbox_envelope = envelopes.first().unwrap();

    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", inbox_envelope.from.addr);
    assert_eq!("Plain message!", inbox_envelope.subject);

    let envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();
    let custom_envelope = envelopes.first().unwrap();

    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", custom_envelope.from.addr);
    assert_eq!("Plain message custom!", custom_envelope.subject);

    // check that the added message exists

    let msgs = notmuch
        .get_messages(INBOX, &Id::single(&*inbox_id))
        .await
        .unwrap();

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

    let msgs = notmuch
        .get_messages("custom", &Id::single(&*custom_id))
        .await
        .unwrap();

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
        "Plain message custom!",
        "",
    );

    assert_eq!(tpl, expected_tpl);

    // check that a flag can be added to envelopes

    let flags = Flags::from_iter([Flag::Flagged, Flag::Answered]);
    notmuch
        .add_flags(INBOX, &Id::single(&*inbox_id), &flags)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    notmuch
        .add_flag("custom", &Id::single(&*custom_id), Flag::Flagged)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that envelopes flags can be changed

    let flags = Flags::from_iter([Flag::custom("flag"), Flag::Answered]);
    notmuch
        .set_flags(INBOX, &Id::single(&*inbox_id), &flags)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    notmuch
        .set_flags("custom", &Id::single(&*custom_id), &flags)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from envelopes

    notmuch
        .remove_flag(INBOX, &Id::single(&*inbox_id), Flag::Answered)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    notmuch
        .remove_flag("custom", &Id::single(&*custom_id), Flag::Answered)
        .await
        .unwrap();

    let envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();

    assert!(!envelope.flags.contains(&Flag::Custom("flag".into())));
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be copied

    notmuch
        .copy_messages(INBOX, "custom", &Id::single(&*inbox_id))
        .await
        .unwrap();

    let inbox_envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let custom_envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();

    assert_eq!(inbox_envelopes.len(), 1);
    assert_eq!(custom_envelopes.len(), 2);

    // check that the message can be moved

    notmuch
        .move_messages("custom", INBOX, &Id::single(&*custom_id))
        .await
        .unwrap();

    let inbox_envelopes = notmuch.list_envelopes(INBOX, 0, 0).await.unwrap();
    let custom_envelopes = notmuch.list_envelopes("custom", 0, 0).await.unwrap();

    assert_eq!(inbox_envelopes.len(), 2);
    assert_eq!(custom_envelopes.len(), 1);
}
