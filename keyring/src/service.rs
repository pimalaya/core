//! # Global service name
//!
//! Module dedicated to global keyring service name management. Every
//! consumer of this crate should define his own service name at the
//! beginning of their program.

use once_cell::sync::OnceCell;
use tracing::debug;

/// The global service name, wrapped in a once cell.
static SERVICE_NAME: OnceCell<&str> = OnceCell::new();

/// The default global service name.
static DEFAULT_SERVICE_NAME: &str = "keyring-lib";

/// Gets the global keyring service name.
///
/// If the service name is not defined, returns the default global
/// service name as defined in [`DEFAULT_SERVICE_NAME`].
pub fn get_global_service_name() -> &'static str {
    match SERVICE_NAME.get() {
        Some(name) => name,
        None => {
            let name = DEFAULT_SERVICE_NAME;
            debug!(name, "undefined global service name, using defaults");
            name
        }
    }
}

/// Replaces the global keyring service name.
///
/// This function has no effect if a global service name has already
/// been defined.
pub fn set_global_service_name(name: &'static str) {
    debug!(name, "define global service name");

    if let Err((prev, _)) = SERVICE_NAME.try_insert(name) {
        debug!(name = prev, "service name already defined, skipping it");
    }
}
