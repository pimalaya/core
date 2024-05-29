pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;

use super::{list::ListEnvelopesOptions, SingleId, ThreadedEnvelopes};
use crate::AnyResult;

#[async_trait]
pub trait ThreadEnvelopes: Send + Sync {
    /// Thread all available envelopes from the given folder matching
    /// the given pagination.
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes>;

    async fn thread_envelope(
        &self,
        _folder: &str,
        _id: SingleId,
        _opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        unimplemented!()
    }
}
