//! Module dedicated to the sendmail sender configuration.
//!
//! This module contains the configuration specific to the sendmail
//! sender.

use process::Cmd;

/// The sendmail sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SendmailConfig {
    /// The sendmail command.
    pub cmd: Cmd,
}
