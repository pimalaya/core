#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeListConfig {
    /// Define the size of a page when listing envelopes.
    ///
    /// A page size of 0 disables the pagination and shows all
    /// available envelopes.
    pub page_size: Option<usize>,

    /// Customize the format for displaying envelopes date.
    ///
    /// See [`chrono::format::strftime`] for supported
    /// formats. Defaults to `%F %R%:z`.
    pub datetime_fmt: Option<String>,

    /// Transform envelopes date timezone into the user's
    /// local one.
    ///
    /// For example, if the user's local timezone is UTC, the envelope
    /// date `2023-06-15T09:00:00+02:00` becomes
    /// `2023-06-15T07:00:00-00:00`.
    pub datetime_local_tz: Option<bool>,
}
