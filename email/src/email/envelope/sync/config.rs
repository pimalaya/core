use chrono::{DateTime, Local};

use crate::envelope::Envelope;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeSyncConfig {
    #[cfg_attr(feature = "derive", serde(default))]
    pub filter: EnvelopeSyncFilters,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeSyncFilters {
    #[cfg_attr(feature = "derive", serde(default))]
    pub date_range: EnvelopeSyncDateRangeFilter,
}

impl EnvelopeSyncFilters {
    pub fn matches(&self, envelope: &Envelope) -> bool {
        self.date_range.matches(envelope)
    }
}

/// The date range synchronization filter.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeSyncDateRangeFilter {
    /// Filter envelopes with a `Date` header more recent than the given
    /// date.
    from: Option<DateTime<Local>>,

    /// Filter envelopes with a `Date` header older than the given date.
    to: Option<DateTime<Local>>,
}

impl EnvelopeSyncDateRangeFilter {
    pub fn set_some_from(&mut self, date: Option<impl Into<DateTime<Local>>>) {
        self.from = date.map(Into::into);
    }

    pub fn set_from(&mut self, date: impl Into<DateTime<Local>>) {
        self.set_some_from(Some(date));
    }

    pub fn with_some_from(mut self, date: Option<impl Into<DateTime<Local>>>) -> Self {
        self.set_some_from(date);
        self
    }

    pub fn with_from(mut self, date: impl Into<DateTime<Local>>) -> Self {
        self.set_from(date);
        self
    }

    pub fn find_from(&self) -> Option<&DateTime<Local>> {
        self.from.as_ref()
    }

    pub fn set_some_to(&mut self, date: Option<impl Into<DateTime<Local>>>) {
        self.to = date.map(Into::into);
    }

    pub fn set_to(&mut self, date: impl Into<DateTime<Local>>) {
        self.set_some_to(Some(date));
    }

    pub fn with_some_to(mut self, date: Option<impl Into<DateTime<Local>>>) -> Self {
        self.set_some_to(date);
        self
    }

    pub fn with_to(mut self, date: impl Into<DateTime<Local>>) -> Self {
        self.set_to(date);
        self
    }

    pub fn find_to(&self) -> Option<&DateTime<Local>> {
        self.to.as_ref()
    }

    pub fn matches(&self, envelope: &Envelope) -> bool {
        let date = envelope.date.with_timezone(&Local);
        self.matches_from(&date) && self.matches_to(&date)
    }

    pub fn matches_from(&self, date: &DateTime<Local>) -> bool {
        match self.from.as_ref() {
            Some(from) => from >= date,
            None => true,
        }
    }

    pub fn matches_to(&self, date: &DateTime<Local>) -> bool {
        match self.to.as_ref() {
            Some(to) => to <= date,
            None => true,
        }
    }
}
