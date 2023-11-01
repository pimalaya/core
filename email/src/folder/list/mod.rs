use async_trait::async_trait;
use std::fmt::Debug;

use crate::Result;

use super::{Folder, Folders};

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait ListFolders: Debug {
    async fn list_folders(&self) -> Result<Folders>;
}
