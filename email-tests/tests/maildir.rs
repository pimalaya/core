use std::{collections::HashMap, iter::FromIterator, sync::Arc};

use concat_with::concat_line;
use email::{
    account::config::AccountConfig,
    backend::BackendBuilder,
    envelope::{list::ListEnvelopes, Id},
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags, Flag},
    folder::{
        add::AddFolder, config::FolderConfig, delete::DeleteFolder, expunge::ExpungeFolder,
        list::ListFolders, Folder, FolderKind, Folders,
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        r#move::MoveMessages,
    },
};
use mail_builder::MessageBuilder;
use tempfile::tempdir;

#[test_log::test(tokio::test)]
async fn test_maildir_features() {
    let tmp_dir = tempdir().unwrap().path().to_owned();

    let account_config = Arc::new(AccountConfig {
        name: "account".into(),
        folder: Some(FolderConfig {
            aliases: Some(HashMap::from_iter([
                ("inbox".into(), "Inbox".into()),
                ("subdir".into(), "Subdir".into()),
                ("subsubdir".into(), "Subdir/Subdir".into()),
            ])),
            ..Default::default()
        }),
        ..Default::default()
    });

    let mdir_config = Arc::new(MaildirConfig {
        root_dir: tmp_dir.clone(),
        maildirpp: false,
    });

    let mdir_ctx = MaildirContextBuilder::new(account_config.clone(), mdir_config.clone());
    let mdir = BackendBuilder::new(account_config.clone(), mdir_ctx)
        .build()
        .await
        .unwrap();

    // testing folders

    mdir.add_folder("inbox").await.unwrap();
    mdir.add_folder("subdir").await.unwrap();
    mdir.add_folder("Subdir/Subdir").await.unwrap();
    mdir.add_folder("Trash").await.unwrap();
    mdir.add_folder("Nested").await.unwrap();
    mdir.add_folder("Nested/Folder").await.unwrap();

    let folders = mdir.list_folders().await.unwrap();
    let expected_folders = Folders::from_iter([
        Folder {
            name: "Inbox".into(),
            kind: Some(FolderKind::Inbox),
            desc: tmp_dir.join("Inbox").to_string_lossy().to_string(),
        },
        Folder {
            name: "Nested".into(),
            kind: None,
            desc: tmp_dir.join("Nested").to_string_lossy().to_string(),
        },
        Folder {
            name: "Nested/Folder".into(),
            kind: None,
            desc: tmp_dir
                .join("Nested")
                .join("Folder")
                .to_string_lossy()
                .to_string(),
        },
        Folder {
            name: "Trash".into(),
            kind: Some(FolderKind::Trash),
            desc: tmp_dir.join("Trash").to_string_lossy().to_string(),
        },
        Folder {
            name: "Subdir".into(),
            kind: Some(FolderKind::UserDefined("subdir".into())),
            desc: tmp_dir.join("Subdir").to_string_lossy().to_string(),
        },
        Folder {
            name: "Subdir/Subdir".into(),
            kind: Some(FolderKind::UserDefined("subsubdir".into())),
            desc: tmp_dir
                .join("Subdir")
                .join("Subdir")
                .to_string_lossy()
                .to_string(),
        },
    ]);

    assert_eq!(folders, expected_folders);

    // deleting a root's nested folders should not delete nested
    // folders
    mdir.delete_folder("Nested").await.unwrap();

    let folders = mdir.list_folders().await.unwrap();
    let expected_folders = Folders::from_iter([
        Folder {
            name: "Inbox".into(),
            kind: Some(FolderKind::Inbox),
            desc: tmp_dir.join("Inbox").to_string_lossy().to_string(),
        },
        Folder {
            name: "Nested/Folder".into(),
            kind: None,
            desc: tmp_dir
                .join("Nested")
                .join("Folder")
                .to_string_lossy()
                .to_string(),
        },
        Folder {
            name: "Trash".into(),
            kind: Some(FolderKind::Trash),
            desc: tmp_dir.join("Trash").to_string_lossy().to_string(),
        },
        Folder {
            name: "Subdir".into(),
            kind: Some(FolderKind::UserDefined("subdir".into())),
            desc: tmp_dir.join("Subdir").to_string_lossy().to_string(),
        },
        Folder {
            name: "Subdir/Subdir".into(),
            kind: Some(FolderKind::UserDefined("subsubdir".into())),
            desc: tmp_dir
                .join("Subdir")
                .join("Subdir")
                .to_string_lossy()
                .to_string(),
        },
    ]);

    assert_eq!(folders, expected_folders);

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
    );

    assert_eq!(*tpl, expected_tpl);

    // check that the envelope of the added message exists
    let envelopes = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let envelope = envelopes.first().unwrap();
    assert_eq!(1, envelopes.len());
    assert_eq!("alice@localhost", envelope.from.addr);
    assert_eq!("Plain message!", envelope.subject);

    // check that a flag can be added to the message
    mdir.add_flag("INBOX", &Id::single(&envelope.id), Flag::Flagged)
        .await
        .unwrap();
    let envelopes = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(envelope.flags.contains(&Flag::Seen));
    assert!(envelope.flags.contains(&Flag::Flagged));

    // check that the message flags can be changed
    mdir.set_flag("INBOX", &Id::single(&envelope.id), Flag::Answered)
        .await
        .unwrap();
    let envelopes = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(envelope.flags.contains(&Flag::Answered));

    // check that a flag can be removed from the message
    mdir.remove_flag("INBOX", &Id::single(&envelope.id), Flag::Answered)
        .await
        .unwrap();
    let envelopes = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let envelope = envelopes.first().unwrap();
    assert!(!envelope.flags.contains(&Flag::Seen));
    assert!(!envelope.flags.contains(&Flag::Flagged));
    assert!(!envelope.flags.contains(&Flag::Answered));

    // check that the message can be copied
    mdir.copy_messages("INBOX", "subdir", &Id::single(&envelope.id))
        .await
        .unwrap();
    let inbox = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let subdir = mdir
        .list_envelopes("subdir", Default::default())
        .await
        .unwrap();
    let trash = mdir
        .list_envelopes("Trash", Default::default())
        .await
        .unwrap();
    assert_eq!(1, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(0, trash.len());
    assert!(mdir
        .get_messages("INBOX", &Id::single(&inbox[0].id))
        .await
        .is_ok());
    assert!(mdir
        .get_messages("subdir", &Id::single(&subdir[0].id))
        .await
        .is_ok());

    // check that the email can be marked as deleted then expunged
    mdir.add_flag("subdir", &Id::single(&subdir[0].id), Flag::Deleted)
        .await
        .unwrap();
    let inbox = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let subdir = mdir
        .list_envelopes("subdir", Default::default())
        .await
        .unwrap();
    let trash = mdir
        .list_envelopes("Trash", Default::default())
        .await
        .unwrap();
    assert_eq!(1, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(0, trash.len());

    mdir.expunge_folder("subdir").await.unwrap();
    let subdir = mdir
        .list_envelopes("subdir", Default::default())
        .await
        .unwrap();
    assert_eq!(0, subdir.len());

    // check that the message can be moved
    mdir.move_messages("INBOX", "subdir", &Id::single(&envelope.id))
        .await
        .unwrap();
    let inbox = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let subdir = mdir
        .list_envelopes("subdir", Default::default())
        .await
        .unwrap();
    let trash = mdir
        .list_envelopes("Trash", Default::default())
        .await
        .unwrap();
    assert_eq!(0, inbox.len());
    assert_eq!(1, subdir.len());
    assert_eq!(0, trash.len());

    // check that the message can be deleted
    mdir.delete_messages("subdir", &Id::single(&subdir[0].id))
        .await
        .unwrap();
    let inbox = mdir
        .list_envelopes("INBOX", Default::default())
        .await
        .unwrap();
    let subdir = mdir
        .list_envelopes("subdir", Default::default())
        .await
        .unwrap();
    let trash = mdir
        .list_envelopes("Trash", Default::default())
        .await
        .unwrap();
    assert_eq!(0, inbox.len());
    assert_eq!(0, subdir.len());
    assert_eq!(1, trash.len());

    mdir.delete_messages("Trash", &Id::single(&trash[0].id))
        .await
        .unwrap();
    let trash = mdir
        .list_envelopes("Trash", Default::default())
        .await
        .unwrap();
    assert_eq!(1, trash.len());
    assert!(trash[0].flags.contains(&Flag::Deleted));

    mdir.expunge_folder("Trash").await.unwrap();
    let trash = mdir
        .list_envelopes("Trash", Default::default())
        .await
        .unwrap();
    assert_eq!(0, trash.len());
}
