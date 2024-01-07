use concat_with::concat_line;
use email::{
    account::config::AccountConfig,
    backend::BackendBuilder,
    envelope::{list::maildir::ListEnvelopesMaildir, Id},
    flag::{
        add::maildir::AddFlagsMaildir, remove::maildir::RemoveFlagsMaildir,
        set::maildir::SetFlagsMaildir, Flag,
    },
    folder::{
        add::maildir::AddFolderMaildir, config::FolderConfig, delete::maildir::DeleteFolderMaildir,
        expunge::maildir::ExpungeFolderMaildir, list::maildir::ListFoldersMaildir,
    },
    maildir::{config::MaildirConfig, MaildirSessionBuilder},
    message::{
        add_with_flags::maildir::AddMessageWithFlagsMaildir, copy::maildir::CopyMessagesMaildir,
        move_::maildir::MoveMessagesMaildir, peek::maildir::PeekMessagesMaildir,
    },
};
use mail_builder::MessageBuilder;
use maildirpp::Maildir;
use std::{collections::HashMap, fs, iter::FromIterator};
use tempfile::tempdir;

#[tokio::test]
async fn test_maildir_features() {
    env_logger::builder().is_test(true).init();

    // set up maildir folders

    let mdir: Maildir = tempdir().unwrap().path().to_owned().into();
    if let Err(_) = fs::remove_dir_all(mdir.path()) {}
    mdir.create_dirs().unwrap();

    let mdir_sub: Maildir = mdir.path().join(".Subdir").into();
    if let Err(_) = fs::remove_dir_all(mdir_sub.path()) {}
    mdir_sub.create_dirs().unwrap();

    let mdir_trash = Maildir::from(mdir.path().join(".Trash"));
    if let Err(_) = fs::remove_dir_all(mdir_trash.path()) {}
    mdir_trash.create_dirs().unwrap();

    let config = AccountConfig {
        name: "account".into(),
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([
                ("subdir".into(), "Subdir".into()),
                (
                    "abs-subdir".into(),
                    mdir.path().join(".Subdir").to_string_lossy().into(),
                ),
            ])),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Main maildir backend

    let mdir_path = mdir.path().to_owned();
    let mdir_config = MaildirConfig {
        root_dir: mdir_path.clone(),
    };
    let backend_ctx = MaildirSessionBuilder::new(config.clone(), mdir_config);
    let backend_builder = BackendBuilder::new(config.clone(), backend_ctx)
        .with_add_folder(AddFolderMaildir::new)
        .with_list_folders(ListFoldersMaildir::new)
        .with_expunge_folder(ExpungeFolderMaildir::new)
        .with_delete_folder(DeleteFolderMaildir::new)
        .with_list_envelopes(ListEnvelopesMaildir::new)
        .with_add_flags(AddFlagsMaildir::new)
        .with_set_flags(SetFlagsMaildir::new)
        .with_remove_flags(RemoveFlagsMaildir::new)
        .with_peek_messages(PeekMessagesMaildir::new)
        .with_add_message_with_flags(AddMessageWithFlagsMaildir::new)
        .with_copy_messages(CopyMessagesMaildir::new)
        .with_move_messages(MoveMessagesMaildir::new);
    let mdir = backend_builder.build().await.unwrap();

    // Sub maildir backend

    let mdir_config = MaildirConfig {
        root_dir: mdir_path.clone(),
    };
    let backend_ctx = MaildirSessionBuilder::new(config.clone(), mdir_config);
    let backend_builder = BackendBuilder::new(config.clone(), backend_ctx)
        .with_add_folder(AddFolderMaildir::new)
        .with_list_folders(ListFoldersMaildir::new)
        .with_expunge_folder(ExpungeFolderMaildir::new)
        .with_delete_folder(DeleteFolderMaildir::new)
        .with_list_envelopes(ListEnvelopesMaildir::new)
        .with_add_flags(AddFlagsMaildir::new)
        .with_set_flags(SetFlagsMaildir::new)
        .with_remove_flags(RemoveFlagsMaildir::new)
        .with_peek_messages(PeekMessagesMaildir::new)
        .with_add_message_with_flags(AddMessageWithFlagsMaildir::new)
        .with_copy_messages(CopyMessagesMaildir::new)
        .with_move_messages(MoveMessagesMaildir::new);
    let submdir = backend_builder.build().await.unwrap();

    // check that a message can be built and added
    let email = MessageBuilder::new()
        .from("alice@localhost")
        .to("bob@localhost")
        .subject("Plain message!")
        .text_body("Plain message!")
        .write_to_vec()
        .unwrap();
    let id = mdir
        .add_message_with_flag("INBOX", &email, Flag::Seen)
        .await
        .unwrap();

    // check that the added message exists
    let emails = mdir.get_messages("INBOX", &id.into()).await.unwrap();
    let tpl = emails
        .to_vec()
        .first()
        .unwrap()
        .to_read_tpl(&config, |i| i.with_show_only_headers(["From", "To"]))
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
    let envelopes = mdir.list_envelopes("INBOX", 10, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);

    // check that a flag can be added to the message
    mdir.add_flag("INBOX", &Id::single(&envelope.id), Flag::Flagged)
        .await
        .unwrap();
    let envelopes = mdir.list_envelopes("INBOX", 1, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));

    // check that the message flags can be changed
    mdir.set_flag("INBOX", &Id::single(&envelope.id), Flag::Answered)
        .await
        .unwrap();
    let envelopes = mdir.list_envelopes("INBOX", 1, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from the message
    mdir.remove_flag("INBOX", &Id::single(&envelope.id), Flag::Answered)
        .await
        .unwrap();
    let envelopes = mdir.list_envelopes("INBOX", 1, 0).await.unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be copied
    mdir.copy_messages("INBOX", "subdir", &Id::single(&envelope.id))
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let abs_subdir = mdir.list_envelopes("abs-subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(1, abs_subdir.len());
    assert_eq!(0, trash.len());
    assert!(mdir
        .get_messages("INBOX", &Id::single(&inbox[0].id))
        .await
        .is_ok());
    assert!(mdir
        .get_messages("subdir", &Id::single(&subdir[0].id))
        .await
        .is_ok());
    assert!(submdir
        .get_messages("INBOX", &Id::single(&subdir[0].id))
        .await
        .is_ok());

    // check that the email can be marked as deleted then expunged
    mdir.add_flag("subdir", &Id::single(&subdir[0].id), Flag::Deleted)
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let abs_subdir = mdir.list_envelopes("abs-subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(1, abs_subdir.len());
    assert_eq!(0, trash.len());

    mdir.expunge_folder("subdir").await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let abs_subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    assert_eq!(0, subdir.len());
    assert_eq!(0, abs_subdir.len());

    // check that the message can be moved
    mdir.move_messages("INBOX", "subdir", &Id::single(&envelope.id))
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(0, trash.len());

    // check that the message can be deleted
    mdir.delete_messages("subdir", &Id::single(&subdir[0].id))
        .await
        .unwrap();
    let inbox = mdir.list_envelopes("INBOX", 0, 0).await.unwrap();
    let subdir = mdir.list_envelopes("subdir", 0, 0).await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, inbox.len());
    assert_eq!(0, subdir.len());
    assert_eq!(1, trash.len());

    mdir.delete_messages("Trash", &Id::single(&trash[0].id))
        .await
        .unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(1, trash.len());
    assert!(trash[0].flags.contains(&Flag::Deleted));

    mdir.expunge_folder("Trash").await.unwrap();
    let trash = mdir.list_envelopes("Trash", 0, 0).await.unwrap();
    assert_eq!(0, trash.len());
}
