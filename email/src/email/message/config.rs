use serde::{Deserialize, Serialize};

use super::{
    add_raw::config::MessageWriteConfig, get::config::MessageReadConfig,
    send_raw::config::MessageSendConfig,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MessageConfig {
    /// The message config related to reading.
    pub read: Option<MessageReadConfig>,

    /// The message config related to writing.
    pub write: Option<MessageWriteConfig>,

    /// The message config related to sending.
    pub send: Option<MessageSendConfig>,
}
