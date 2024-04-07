use async_trait::async_trait;
use log::{debug, info, trace};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{collections::HashMap, sync::mpsc};

use crate::{
    email::error::Error,
    envelope::{Envelope, Envelopes},
    maildir::MaildirContextSync,
    AnyResult,
};

use super::WatchEnvelopes;

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
    async fn watch_envelopes(&self, folder: &str) -> AnyResult<()> {
        info!("maildir: watching folder {folder} for email changes");

        let session = self.ctx.lock().await;
        let config = &session.account_config;

        let mdir = session.get_maildir_from_folder_name(folder)?;
        let envelopes = Envelopes::from_mdir_entries(mdir.list_cur(), None);
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
                Ok(evt) => {
                    trace!("received filesystem change event: {evt:?}");

                    let next_envelopes = Envelopes::from_mdir_entries(mdir.list_cur(), None);
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
