//! Module dedicated to sender configuration.
//!
//! This module contains the sender configuration used for the current
//! account. One account can have only one sender and so one sender
//! configuration.

use crate::SendmailConfig;
#[cfg(feature = "smtp-sender")]
use crate::SmtpConfig;

/// The sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum SenderConfig {
    /// The undefined sender is useful when you need to create an
    /// account that only manipulates emails using a [crate::Backend].
    #[default]
    None,

    /// The SMTP sender configuration.
    #[cfg(feature = "smtp-sender")]
    Smtp(SmtpConfig),

    /// The sendmail configuration.
    Sendmail(SendmailConfig),
}
