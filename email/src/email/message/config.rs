#[cfg(feature = "sync")]
use super::sync::config::MessageSyncConfig;
use super::{
    add::config::MessageWriteConfig, delete::config::DeleteMessageConfig,
    get::config::MessageReadConfig, send::config::MessageSendConfig,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct MessageConfig {
    /// Configuration dedicated to message reading.
    pub read: Option<MessageReadConfig>,

    /// Configuration dedicated to message writing.
    pub write: Option<MessageWriteConfig>,

    /// Configuration dedicated to message sending.
    pub send: Option<MessageSendConfig>,

    /// Configuration dedicated to message deletion.
    pub delete: Option<DeleteMessageConfig>,

    #[cfg(feature = "sync")]
    /// Configuration dedicated to message sending.
    pub sync: Option<MessageSyncConfig>,
}
