use async_trait::async_trait;
use futures::StreamExt;
use inotify::{Inotify, WatchMask};
use log::{debug, info};
use std::{io, path::PathBuf};
use thiserror::Error;

use crate::{account::config::DEFAULT_INBOX_FOLDER, maildir::MaildirSessionSync, Result};

use super::WatchFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderError(PathBuf, PathBuf),
    #[error("cannot create maildir {1} folder structure")]
    InitFolderError(#[source] maildirpp::Error, PathBuf),

    #[error(transparent)]
    InotifyError(#[from] io::Error),
}

pub struct WatchFolderMaildir {
    session: MaildirSessionSync,
}

impl WatchFolderMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn WatchFolder>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl WatchFolder for WatchFolderMaildir {
    async fn watch_folder(&self, folder: &str) -> Result<()> {
        info!("maildir: watching folder {folder}");

        let session = self.session.lock().await;

        // FIXME: better check if given folder IS the inbox
        let path = match session.account_config.get_folder_alias(folder)?.as_str() {
            DEFAULT_INBOX_FOLDER => session.path().join("cur"),
            folder => {
                let folder = session.encode_folder(folder);
                session.path().join(format!(".{}", folder))
            }
        };

        let inotify = Inotify::init()?;

        inotify.watches().add(
            path,
            WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE,
        )?;

        let mut buffer = [0; 1024];
        let mut stream = inotify.into_event_stream(&mut buffer)?;

        while let Some(res) = stream.next().await {
            match res {
                Ok(evt) => {
                    debug!("received inotify event: {evt:?}");
                    session.account_config.run_folder_watch_change_hooks().await;
                }
                Err(err) => {
                    debug!("error while receiving inotify event: {err}");
                    debug!("{err:?}");
                }
            }
        }

        Ok(())
    }
}
