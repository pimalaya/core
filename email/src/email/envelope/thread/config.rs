#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeThreadConfig {
    /// Define the size of a page when threading envelopes.
    ///
    /// A page size of 0 disables the pagination and shows all
    /// available envelopes.
    pub page_size: Option<usize>,
}
