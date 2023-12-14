use serde::{Deserialize, Serialize};

use super::{
    add_raw::config::MessageWriteConfig, get::config::MessageReadConfig,
    send_raw::config::MessageSendConfig, watch::config::WatchMessageConfig,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MessageConfig {
    /// Configuration dedicated to message reading.
    pub read: Option<MessageReadConfig>,

    /// Configuration dedicated to message writing.
    pub write: Option<MessageWriteConfig>,

    /// Configuration dedicated to message sending.
    pub send: Option<MessageSendConfig>,

    /// Configuration dedicated to message changes.
    pub watch: Option<WatchMessageConfig>,
}
