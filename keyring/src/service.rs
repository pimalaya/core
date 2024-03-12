//! # Keyring service name
//!
//! Module dedicated to global service name management. Every consumer
//! of this crate should define his own service name.

use log::debug;
use once_cell::sync::OnceCell;

/// The global service name, wrapped in a once cell.
static SERVICE_NAME: OnceCell<&str> = OnceCell::with_value("keyring-lib");

/// Get the global keyring service name.
///
/// The service name is used every time a new native entry is created.
pub fn get_global_service_name() -> &'static str {
    SERVICE_NAME.get().unwrap()
}

/// Replace the global keyring service name.
pub fn set_global_service_name(next: &'static str) {
    if let Err(prev) = SERVICE_NAME.set(next) {
        debug!("global keyring service name {prev} replaced by {next}");
    } else {
        debug!("global keyring service name set to {next}");
    }
}
