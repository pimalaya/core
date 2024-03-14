//! Module dedicated to the sendmail sender configuration.
//!
//! This module contains the configuration specific to the sendmail
//! sender.

use process::Cmd;

/// The sendmail sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct SendmailConfig {
    /// The sendmail command.
    pub cmd: Cmd,
}
