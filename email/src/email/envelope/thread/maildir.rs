use async_trait::async_trait;
use tracing::instrument;

use super::{build_thread_graph_all, build_thread_graph_for_id, ThreadEnvelopes};
use crate::{
    envelope::{
        list::ListEnvelopesOptions, Envelopes, SingleId, ThreadedEnvelopes,
    },
    maildir::MaildirContextSync,
    AnyResult, Error,
};

#[derive(Clone)]
pub struct ThreadMaildirEnvelopes {
    ctx: MaildirContextSync,
}

impl ThreadMaildirEnvelopes {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn ThreadEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ThreadEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ThreadEnvelopes for ThreadMaildirEnvelopes {
    #[instrument(skip(self, opts))]
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entries = mdir.read().map_err(Error::MaildirsError)?;
        let envelopes = Envelopes::from_mdir_entries(entries, opts.query.as_ref())
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();

        let envelopes = ThreadedEnvelopes::new(envelopes, build_thread_graph_all);

        Ok(envelopes)
    }

    #[instrument(skip(self, opts))]
    async fn thread_envelope(
        &self,
        folder: &str,
        id: SingleId,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entries = mdir.read().map_err(Error::MaildirsError)?;
        let envelopes = Envelopes::from_mdir_entries(entries, opts.query.as_ref())
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();

        let envelopes =
            ThreadedEnvelopes::new(envelopes, move |envelopes| {
                build_thread_graph_for_id(envelopes, id.as_str())
            });

        Ok(envelopes)
    }
}
