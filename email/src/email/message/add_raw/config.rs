use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MessageWriteConfig {
    /// Define visible headers at the top of messages when writing
    /// them (new/reply/forward).
    pub headers: Option<Vec<String>>,
}
