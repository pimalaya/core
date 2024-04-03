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
    /// Filter envelopes with a `Date` header more recent than the given
    /// date.
    pub before: Option<DateTime<Local>>,

    /// Filter envelopes with a `Date` header older than the given date.
    pub after: Option<DateTime<Local>>,
}

impl EnvelopeSyncFilters {
    pub fn set_some_after(&mut self, date: Option<impl Into<DateTime<Local>>>) {
        self.before = date.map(Into::into);
    }

    pub fn set_after(&mut self, date: impl Into<DateTime<Local>>) {
        self.set_some_after(Some(date));
    }

    pub fn with_some_after(mut self, date: Option<impl Into<DateTime<Local>>>) -> Self {
        self.set_some_after(date);
        self
    }

    pub fn with_after(mut self, date: impl Into<DateTime<Local>>) -> Self {
        self.set_after(date);
        self
    }

    pub fn after(&self) -> Option<&DateTime<Local>> {
        self.before.as_ref()
    }

    pub fn set_some_before(&mut self, date: Option<impl Into<DateTime<Local>>>) {
        self.after = date.map(Into::into);
    }

    pub fn set_before(&mut self, date: impl Into<DateTime<Local>>) {
        self.set_some_before(Some(date));
    }

    pub fn with_some_before(mut self, date: Option<impl Into<DateTime<Local>>>) -> Self {
        self.set_some_before(date);
        self
    }

    pub fn with_before(mut self, date: impl Into<DateTime<Local>>) -> Self {
        self.set_before(date);
        self
    }

    pub fn before(&self) -> Option<&DateTime<Local>> {
        self.after.as_ref()
    }

    pub fn matches(&self, envelope: &Envelope) -> bool {
        let date = envelope.date.with_timezone(&Local);
        self.matches_after(&date) && self.matches_before(&date)
    }

    pub fn matches_after(&self, date: &DateTime<Local>) -> bool {
        match self.after() {
            Some(after) => after > date,
            None => true,
        }
    }

    pub fn matches_before(&self, date: &DateTime<Local>) -> bool {
        match self.before() {
            Some(before) => before < date,
            None => true,
        }
    }
}
