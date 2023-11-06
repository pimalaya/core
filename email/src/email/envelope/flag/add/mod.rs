use async_trait::async_trait;

use crate::{email::envelope::Id, Result};

use super::{Flag, Flags};

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddFlag: Send + Sync {
    /// Add the given flag to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()>;
}

#[async_trait]
pub trait AddFlags: Send + Sync {
    /// Add the given flags to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()>;
}

#[async_trait]
impl<T: AddFlags> AddFlag for T {
    /// Add the given flag to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.add_flags(folder, id, &Flags::from_iter([flag])).await
    }
}
