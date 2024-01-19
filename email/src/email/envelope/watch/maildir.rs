use async_trait::async_trait;
use log::{debug, info, trace};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{collections::HashMap, path::PathBuf, sync::mpsc};
use thiserror::Error;

use crate::{
    envelope::{Envelope, Envelopes},
    maildir::MaildirSession,
    Result,
};

use super::WatchEnvelopes;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderError(PathBuf, PathBuf),
    #[error("cannot create maildir {1} folder structure")]
    InitFolderError(#[source] maildirpp::Error, PathBuf),
}

pub struct WatchMaildirEnvelopes {
    session: MaildirSession,
}

impl WatchMaildirEnvelopes {
    pub fn new(session: &MaildirSession) -> Option<Box<dyn WatchEnvelopes>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl WatchEnvelopes for WatchMaildirEnvelopes {
    async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        info!("maildir: watching folder {folder} for email changes");

        let session = self.session.lock().await;
        let config = &session.account_config;

        let mdir = session.get_maildir_from_folder_name(folder)?;
        let envelopes = Envelopes::from_mdir_entries(mdir.list_cur());
        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Default::default())?;
        watcher.watch(mdir.path(), RecursiveMode::Recursive)?;
        debug!("watching maildir folder {folder:?}…");

        for res in rx {
            match res {
                Ok(evt) => {
                    trace!("received filesystem change event: {evt:?}");

                    let next_envelopes = Envelopes::from_mdir_entries(mdir.list_cur());
                    let next_envelopes: HashMap<String, Envelope> =
                        HashMap::from_iter(next_envelopes.into_iter().map(|e| (e.id.clone(), e)));

                    self.exec_hooks(config, &envelopes, &next_envelopes).await;

                    envelopes = next_envelopes;
                }
                Err(err) => {
                    debug!("error while receiving message added event: {err}");
                    debug!("{err:?}");
                }
            }
        }

        Ok(())
    }
}
