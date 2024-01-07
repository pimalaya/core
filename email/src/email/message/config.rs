use serde::{Deserialize, Serialize};

#[cfg(feature = "message-add")]
use super::add::config::MessageWriteConfig;
#[cfg(feature = "message-get")]
use super::get::config::MessageReadConfig;
#[cfg(feature = "message-send")]
use super::send::config::MessageSendConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageConfig {
    #[cfg(feature = "message-get")]
    /// Configuration dedicated to message reading.
    pub read: Option<MessageReadConfig>,

    #[cfg(feature = "message-add")]
    /// Configuration dedicated to message writing.
    pub write: Option<MessageWriteConfig>,

    #[cfg(feature = "message-send")]
    /// Configuration dedicated to message sending.
    pub send: Option<MessageSendConfig>,
}
