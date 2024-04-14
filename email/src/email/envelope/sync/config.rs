use chrono::NaiveDate;

use crate::search_query::filter::SearchEmailsFilterQuery;

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
    pub before: Option<NaiveDate>,

    /// Filter envelopes with a `Date` header older than the given date.
    pub after: Option<NaiveDate>,
}

impl EnvelopeSyncFilters {
    pub fn set_some_after(&mut self, date: Option<impl Into<NaiveDate>>) {
        self.after = date.map(Into::into);
    }

    pub fn set_after(&mut self, date: impl Into<NaiveDate>) {
        self.set_some_after(Some(date));
    }

    pub fn with_some_after(mut self, date: Option<impl Into<NaiveDate>>) -> Self {
        self.set_some_after(date);
        self
    }

    pub fn with_after(mut self, date: impl Into<NaiveDate>) -> Self {
        self.set_after(date);
        self
    }

    pub fn set_some_before(&mut self, date: Option<impl Into<NaiveDate>>) {
        self.before = date.map(Into::into);
    }

    pub fn set_before(&mut self, date: impl Into<NaiveDate>) {
        self.set_some_before(Some(date));
    }

    pub fn with_some_before(mut self, date: Option<impl Into<NaiveDate>>) -> Self {
        self.set_some_before(date);
        self
    }

    pub fn with_before(mut self, date: impl Into<NaiveDate>) -> Self {
        self.set_before(date);
        self
    }
}

impl From<EnvelopeSyncFilters> for Option<SearchEmailsFilterQuery> {
    fn from(f: EnvelopeSyncFilters) -> Self {
        match (f.before, f.after) {
            (None, None) => None,
            (Some(before), None) => Some(SearchEmailsFilterQuery::BeforeDate(before)),
            (None, Some(after)) => Some(SearchEmailsFilterQuery::AfterDate(after)),
            (Some(before), Some(after)) => Some(SearchEmailsFilterQuery::And(
                Box::new(SearchEmailsFilterQuery::BeforeDate(before)),
                Box::new(SearchEmailsFilterQuery::AfterDate(after)),
            )),
        }
    }
}
