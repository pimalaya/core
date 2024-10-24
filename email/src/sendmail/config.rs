//! Module dedicated to the sendmail sender configuration.
//!
//! This module contains the configuration specific to the sendmail
//! sender.

use once_cell::sync::Lazy;
use process::Command;

pub static SENDMAIL_DEFAULT_COMMAND: Lazy<Command> =
    Lazy::new(|| Command::new("/usr/bin/sendmail"));

/// The sendmail sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct SendmailConfig {
    /// The sendmail command.
    pub cmd: Option<Command>,
}

impl SendmailConfig {
    pub fn cmd(&self) -> &Command {
        self.cmd.as_ref().unwrap_or(&*SENDMAIL_DEFAULT_COMMAND)
    }
}
