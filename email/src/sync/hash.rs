//! # Sync hash
//!
//! Module dedicated to synchronization hashing.

use std::hash::DefaultHasher;

pub trait SyncHash {
    fn sync_hash(&self, state: &mut DefaultHasher);
}
