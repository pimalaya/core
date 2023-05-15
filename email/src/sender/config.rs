use crate::SendmailConfig;
#[cfg(feature = "smtp-sender")]
use crate::SmtpConfig;

/// Represents the email sender provider.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum SenderConfig {
    #[default]
    None,
    #[cfg(feature = "smtp-sender")]
    /// Represents the internal SMTP mailer library.
    Smtp(SmtpConfig),
    /// Represents the sendmail command.
    Sendmail(SendmailConfig),
}
