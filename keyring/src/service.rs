//! # Keyring service name
//!
//! Module dedicated to global keyring service name management. Every
//! consumer of this crate should define his own service name at the
//! beginning of their program.

use log::debug;
use once_cell::sync::OnceCell;

/// The global service name, wrapped in a once cell.
static SERVICE_NAME: OnceCell<&str> = OnceCell::new();

/// The default global service name.
static DEFAULT_SERVICE_NAME: &str = "keyring-lib";

/// Get the global keyring service name.
///
/// If the service name is not defined, returns the default global
/// service name `keyring-lib`.
pub fn get_global_service_name() -> &'static str {
    match SERVICE_NAME.get() {
        Some(name) => name,
        None => {
            let err = format!("service name not defined, defaults to `{DEFAULT_SERVICE_NAME}`");
            debug!("cannot get global keyring service name: {err}");
            DEFAULT_SERVICE_NAME
        }
    }
}

/// Replace the global keyring service name.
///
/// This action as no effect if a global service name has already been
/// defined.
pub fn set_global_service_name(name: &'static str) {
    debug!("setting global keyring service name `{name}`");

    if let Err((prev, _)) = SERVICE_NAME.try_insert(name) {
        let err = format!("service already named `{prev}`");
        debug!("cannot set `{name}` as global keyring service name: {err}");
    }
}
