use imap_client::imap_next::imap_types::{
    core::{Atom, QuotedChar},
    flag::FlagNameAttribute,
    mailbox::Mailbox,
};
use tracing::debug;
use utf7_imap::decode_utf7_imap as decode_utf7;

use super::{Error, FolderKind, Result};
use crate::{
    account::config::AccountConfig,
    folder::{Folder, Folders},
};

pub type ImapMailboxes = Vec<ImapMailbox>;

impl Folders {
    pub fn from_imap_mailboxes(config: &AccountConfig, mboxes: ImapMailboxes) -> Self {
        mboxes
            .into_iter()
            .filter_map(|mbox| match Folder::try_from_imap_mailbox(config, &mbox) {
                Ok(folder) => Some(folder),
                Err(_err) => {
                    debug!("skipping IMAP mailbox {:?}: {_err}", mbox.0.clone());
                    None
                }
            })
            .collect()
    }
}

pub type ImapMailbox = (
    Mailbox<'static>,
    Option<QuotedChar>,
    Vec<FlagNameAttribute<'static>>,
);

impl Folder {
    fn try_from_imap_mailbox(
        config: &AccountConfig,
        (mbox, _delim, attrs): &ImapMailbox,
    ) -> Result<Self> {
        let mbox = match mbox {
            Mailbox::Inbox => String::from("INBOX"),
            Mailbox::Other(mbox) => String::from_utf8_lossy(mbox.as_ref()).to_string(),
        };

        // exit straight if the mailbox is not selectable.
        // TODO: make this behaviour customizable?
        if attrs.contains(&FlagNameAttribute::Noselect) {
            return Err(Error::ParseImapFolderNotSelectableError(mbox.clone()));
        }

        let name = decode_utf7(mbox.into());

        let kind = config
            .find_folder_kind_from_alias(&name)
            .or_else(|| find_folder_kind_from_imap_attrs(attrs.as_ref()))
            .or_else(|| name.parse().ok());

        let desc = attrs.iter().fold(String::default(), |mut desc, attr| {
            if !desc.is_empty() {
                desc.push_str(", ")
            }
            desc.push_str(&format!("{attr}"));
            desc
        });

        Ok(Folder { kind, name, desc })
    }
}

pub fn find_folder_kind_from_imap_attrs(attrs: &[FlagNameAttribute]) -> Option<FolderKind> {
    attrs.iter().find_map(|attr| {
        if attr == &FlagNameAttribute::from(Atom::try_from("Sent").unwrap()) {
            Some(FolderKind::Sent)
        } else if attr == &FlagNameAttribute::from(Atom::try_from("Drafts").unwrap()) {
            Some(FolderKind::Drafts)
        } else if attr == &FlagNameAttribute::from(Atom::try_from("Trash").unwrap()) {
            Some(FolderKind::Trash)
        } else {
            None
        }
    })
}
