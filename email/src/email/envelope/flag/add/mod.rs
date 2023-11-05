use async_trait::async_trait;

use crate::{email::envelope::Id, Result};

use super::Flag;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddFlag: Send + Sync {
    /// Add the given flag to the envelope matching the given id from
    /// the given folder.
    async fn add_flag(&mut self, folder: &str, id: Id, flag: Flag) -> Result<()>;
}
