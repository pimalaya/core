use async_trait::async_trait;
use imap_proto::NameAttribute;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::decode_utf7_imap as decode_utf7;

use crate::{boxed_err, imap::ImapSessionSync, Result};

use super::{Folder, Folders, ListFolders};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot list imap folders")]
    ListFoldersError(#[source] imap::Error),
}

#[derive(Debug)]
pub struct ListFoldersImap {
    session: ImapSessionSync,
}

impl ListFoldersImap {
    pub fn new(session: &ImapSessionSync) -> Box<dyn ListFolders> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl ListFolders for ListFoldersImap {
    async fn list_folders(&self) -> Result<Folders> {
        info!("listing imap folders");

        let mut session = self.session.lock().await;

        let folders = session
            .execute(
                |session| session.list(Some(""), Some("*")),
                |err| boxed_err(Error::ListFoldersError(err)),
            )
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
