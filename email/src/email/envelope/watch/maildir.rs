use std::{collections::HashMap, sync::mpsc};

use async_trait::async_trait;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::oneshot::{Receiver, Sender};
use tracing::{debug, info, trace};

use super::WatchEnvelopes;
use crate::{
    email::error::Error,
    envelope::{Envelope, Envelopes},
    maildir::MaildirContextSync,
    AnyResult,
};

pub struct WatchMaildirEnvelopes {
    ctx: MaildirContextSync,
}

impl WatchMaildirEnvelopes {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn WatchEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn WatchEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl WatchEnvelopes for WatchMaildirEnvelopes {
    async fn watch_envelopes(
        &self,
        folder: &str,
        _wait_for_shutdown_request: Receiver<()>,
        _shutdown: Sender<()>,
    ) -> AnyResult<()> {
        info!("maildir: watching folder {folder} for email changes");

        let session = self.ctx.lock().await;
        let config = &session.account_config;

        let mdir = session.get_maildir_from_folder_alias(folder)?;
        let entries = mdir.read().map_err(Error::MaildirsError)?;
        let envelopes = Envelopes::from_mdir_entries(entries, None);
        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        let (tx, rx) = mpsc::channel();
        let mut watcher =
            RecommendedWatcher::new(tx, Default::default()).map_err(Error::NotifyFailure)?;
        watcher
            .watch(mdir.path(), RecursiveMode::Recursive)
            .map_err(Error::NotifyFailure)?;
        debug!("watching maildir folder {folder:?}…");

        for res in rx {
            match res {
                Ok(_evt) => {
                    trace!("received filesystem change event: {_evt:?}");

                    let entries = mdir.read().map_err(Error::MaildirsError)?;
                    let next_envelopes = Envelopes::from_mdir_entries(entries, None);
                    let next_envelopes: HashMap<String, Envelope> =
                        HashMap::from_iter(next_envelopes.into_iter().map(|e| (e.id.clone(), e)));

                    self.exec_hooks(config, &envelopes, &next_envelopes).await;

                    envelopes = next_envelopes;
                }
                Err(_err) => {
                    debug!("error while receiving message added event: {_err}");
                    debug!("{_err:?}");
                }
            }
        }

        Ok(())
    }
}
