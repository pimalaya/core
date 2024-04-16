use crate::{debug, trace};
use imap::types::{Name, Names};
use imap_proto::NameAttribute;
use utf7_imap::decode_utf7_imap as decode_utf7;

use crate::{
    account::config::AccountConfig,
    folder::{Folder, Folders},
};

use super::FolderKind;

impl Folder {
    /// Parse a folder from an IMAP name.
    ///
    /// Returns [`None`] if the folder cannot be selected.
    pub fn from_imap_name(config: &AccountConfig, name: &Name) -> Option<Self> {
        let attrs = name.attributes();

        // exit straight if the folder cannot be selected
        // TODO: make this behaviour customizable?
        if attrs.contains(&NameAttribute::NoSelect) {
            debug!("skipping not selectable imap folder: {}", name.name());
            return None;
        }

        let name = decode_utf7(name.name().into());

        let kind = config
            .find_folder_kind_from_alias(&name)
            .or_else(|| find_folder_kind_from_imap_attrs(attrs))
            .or_else(|| name.parse().ok());

        let desc = attrs.iter().fold(String::default(), |mut desc, attr| {
            let attr = match attr {
                NameAttribute::All => Some("All"),
                NameAttribute::Archive => Some("Archive"),
                NameAttribute::Flagged => Some("Flagged"),
                NameAttribute::Junk => Some("Junk"),
                NameAttribute::Marked => Some("Marked"),
                NameAttribute::Unmarked => Some("Unmarked"),
                NameAttribute::Extension(ext) => Some(ext.as_ref()),
                _ => None,
            };

            if let Some(attr) = attr {
                if !desc.is_empty() {
                    desc.push_str(", ")
                }
                desc.push_str(attr);
            }

            desc
        });

        let folder = Folder { kind, name, desc };
        trace!("parsed imap folder: {folder:#?}");
        Some(folder)
    }
}

impl Folders {
    /// Parse folders from IMAP names.
    pub fn from_imap_names(config: &AccountConfig, names: Names) -> Self {
        names
            .iter()
            .filter_map(|name| Folder::from_imap_name(config, name))
            .collect()
    }
}

pub fn find_folder_kind_from_imap_attrs(attrs: &[NameAttribute]) -> Option<FolderKind> {
    if attrs.contains(&NameAttribute::Sent) {
        Some(FolderKind::Sent)
    } else if attrs.contains(&NameAttribute::Drafts) {
        Some(FolderKind::Drafts)
    } else if attrs.contains(&NameAttribute::Trash) {
        Some(FolderKind::Trash)
    } else {
        None
    }
}
