#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct MessageWriteConfig {
    /// Define visible headers at the top of messages when writing
    /// them (new/reply/forward).
    pub headers: Option<Vec<String>>,
}
