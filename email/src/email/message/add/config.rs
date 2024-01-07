use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageWriteConfig {
    /// Define visible headers at the top of messages when writing
    /// them (new/reply/forward).
    pub headers: Option<Vec<String>>,
}
