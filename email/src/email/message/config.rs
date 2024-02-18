use serde::{Deserialize, Serialize};

use super::{
    add::config::MessageWriteConfig, get::config::MessageReadConfig,
    send::config::MessageSendConfig,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageConfig {
    /// Configuration dedicated to message reading.
    pub read: Option<MessageReadConfig>,

    /// Configuration dedicated to message writing.
    pub write: Option<MessageWriteConfig>,

    /// Configuration dedicated to message sending.
    pub send: Option<MessageSendConfig>,
}
