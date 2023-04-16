//! Folder module.
//!
//! This module contains everything related to email folders.

pub mod folder;
pub mod folders;
pub mod sync;

pub use self::folder::*;
pub use self::folders::*;
pub use self::sync::SyncBuilder;
