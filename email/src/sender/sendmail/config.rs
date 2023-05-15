//! Sendmail config module.
//!
//! This module contains the representation of the Sendmail email
//! sender configuration of the user account.

use pimalaya_process::Cmd;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SendmailConfig {
    /// Represents the sendmail command.
    pub cmd: Cmd,
}
