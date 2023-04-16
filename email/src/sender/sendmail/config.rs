//! Sendmail config module.
//!
//! This module contains the representation of the Sendmail email
//! sender configuration of the user account.

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct SendmailConfig {
    /// Represents the sendmail command.
    pub cmd: String,
}
