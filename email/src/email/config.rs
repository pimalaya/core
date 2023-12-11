//! Module dedicated to email configuration.
//!
//! This module contains structs related to email configuration. They
//! are mostly used by [crate::AccountConfig].

use process::Cmd;
use serde::{Deserialize, Serialize};

/// The email text/plain format configuration.
///
/// Represents the email text/plain format as defined in the
/// [RFC2646](https://www.ietf.org/rfc/rfc2646.txt).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum EmailTextPlainFormat {
    #[default]
    /// The content should fit its container.
    Auto,
    /// The content should not be restricted.
    Flowed,
    /// The content should fit in a fixed amount of pixels.
    Fixed(usize),
}

impl EmailTextPlainFormat {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// The email hooks configuration.
///
/// Represents the email hooks configuration. They can be useful for
/// doing post and pre processing.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EmailHooks {
    /// The hook called just before sending an email. The system
    /// command should take the raw message as a unique parameter and
    /// returns the modified raw message.
    pub pre_send: Option<Cmd>,
}

impl EmailHooks {
    pub fn is_empty(&self) -> bool {
        self.pre_send.is_none()
    }
}
