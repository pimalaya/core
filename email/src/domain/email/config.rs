//! Email config module.
//!
//! This module contains structures related to email configuration.

use crate::SendmailConfig;
#[cfg(feature = "smtp-sender")]
use crate::SmtpConfig;

/// Represents the email sender provider.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum EmailSender {
    #[default]
    None,
    #[cfg(feature = "smtp-sender")]
    /// Represents the internal SMTP mailer library.
    Smtp(SmtpConfig),
    /// Represents the sendmail command.
    Sendmail(SendmailConfig),
}

/// Represents the text/plain format as defined in the [RFC2646].
///
/// [RFC2646]: https://www.ietf.org/rfc/rfc2646.txt
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum EmailTextPlainFormat {
    #[default]
    /// Makes the content fit its container.
    Auto,
    /// Does not restrict the content.
    Flowed,
    /// Forces the content width with a fixed amount of pixels.
    Fixed(usize),
}

/// Represents the email hooks. Useful for doing extra email
/// processing before or after sending it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EmailHooks {
    /// Represents the hook called just before sending an email.
    pub pre_send: Option<String>,
}

impl EmailHooks {
    pub fn is_empty(&self) -> bool {
        self.pre_send.is_none()
    }
}