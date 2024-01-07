#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;

#[cfg(feature = "flag-add")]
use crate::{envelope::Id, flag::add::AddFlags};
use crate::{
    envelope::SingleId,
    flag::{Flag, Flags},
    Result,
};

#[cfg(feature = "flag-add")]
use super::add::AddMessage;

#[async_trait]
pub trait AddMessageWithFlags: Send + Sync {
    /// Add the given raw email message with the given flags to the
    /// given folder.
    async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId>;

    /// Add the given raw email message with the given flag to the
    /// given folder.
    async fn add_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<SingleId> {
        self.add_message_with_flags(folder, raw_msg, &Flags::from_iter([flag]))
            .await
    }
}

#[cfg(feature = "flag-add")]
#[async_trait]
impl<T: AddMessage + AddFlags> AddMessageWithFlags for T {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        default_add_message_with_flags(self, self, folder, raw_msg, flags).await
    }
}

#[cfg(feature = "flag-add")]
pub async fn default_add_message_with_flags(
    a: &dyn AddMessage,
    b: &dyn AddFlags,
    folder: &str,
    raw_msg: &[u8],
    flags: &Flags,
) -> Result<SingleId> {
    let id = a.add_message(folder, raw_msg).await?;
    b.add_flags(folder, &Id::from(&id), flags).await?;
    Ok(id)
}

#[cfg(feature = "flag-add")]
pub async fn default_add_message_with_flag(
    a: &dyn AddMessage,
    b: &dyn AddFlags,
    folder: &str,
    raw_msg: &[u8],
    flag: Flag,
) -> Result<SingleId> {
    let id = a.add_message(folder, raw_msg).await?;
    b.add_flag(folder, &Id::from(&id), flag).await?;
    Ok(id)
}
