use crate::email::config::EmailTextPlainFormat;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct MessageReadConfig {
    /// Define visible headers at the top of messages when reading
    /// them.
    pub headers: Option<Vec<String>>,

    /// Define the text/plain format as defined in the [RFC
    /// 2646](https://www.ietf.org/rfc/rfc2646.txt).
    pub format: Option<EmailTextPlainFormat>,
}
