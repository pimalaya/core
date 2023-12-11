//! Module dedicated to the sendmail sender configuration.
//!
//! This module contains the configuration specific to the sendmail
//! sender.

use process::Cmd;
use serde::{Deserialize, Serialize};

/// The sendmail sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SendmailConfig {
    /// The sendmail command.
    pub cmd: Cmd,
}
