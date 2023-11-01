use std::sync::Arc;

use async_trait::async_trait;
use imap_proto::NameAttribute;
use log::{debug, info};
use tokio::sync::Mutex;
use utf7_imap::decode_utf7_imap as decode_utf7;

use crate::{imap::ImapSessionManager, Result};

use super::{Folder, Folders, ListFolders};

#[derive(Debug)]
pub struct ListImapFolders {
    session_manager: Arc<Mutex<ImapSessionManager>>,
}

impl ListImapFolders {
    pub fn new(session_manager: Arc<Mutex<ImapSessionManager>>) -> Box<dyn ListFolders> {
        Box::new(Self { session_manager })
    }
}

#[async_trait]
impl ListFolders for ListImapFolders {
    async fn list_folders(&self) -> Result<Folders> {
        info!("listing imap folders");

        let folders = self
            .session_manager
            .lock()
            .await
            .execute(|session| session.list(Some(""), Some("*")))
            .await?;

        let folders = Folders::from_iter(folders.iter().filter_map(|folder| {
            if folder.attributes().contains(&NameAttribute::NoSelect) {
                None
            } else {
                Some(Folder {
                    name: decode_utf7(folder.name().into()),
                    desc: folder
                        .attributes()
                        .iter()
                        .map(|attr| format!("{attr:?}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                })
            }
        }));

        debug!("imap folders: {folders:#?}");

        Ok(folders)
    }
}
