//! # Search emails sort query
//!
//! This module exposes the [`SearchEmailsQuerySorter`] structure,
//! which allows you to sort emails. A sort query can be parsed from a
//! sort query string, see the [`parser`] module for more details.

pub mod parser;

/// The search emails sort query.
///
/// The sorter query is composed of a kind (date, from, to, subject)
/// and an order (ascending, descending).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuerySorter(
    /// The search emails sorter kind.
    pub SearchEmailsQuerySorterKind,
    /// The search emails sorter order.
    pub SearchEmailsQueryOrder,
);

impl SearchEmailsQuerySorter {
    /// Create a new search emails sorter from a kind and an order.
    pub fn new(kind: SearchEmailsQuerySorterKind, order: SearchEmailsQueryOrder) -> Self {
        Self(kind, order)
    }
}

impl From<(SearchEmailsQuerySorterKind, SearchEmailsQueryOrder)> for SearchEmailsQuerySorter {
    fn from((kind, order): (SearchEmailsQuerySorterKind, SearchEmailsQueryOrder)) -> Self {
        SearchEmailsQuerySorter::new(kind, order)
    }
}

impl From<(SearchEmailsQuerySorterKind, Option<SearchEmailsQueryOrder>)>
    for SearchEmailsQuerySorter
{
    fn from((kind, order): (SearchEmailsQuerySorterKind, Option<SearchEmailsQueryOrder>)) -> Self {
        (kind, order.unwrap_or_default()).into()
    }
}

impl From<SearchEmailsQuerySorterKind> for SearchEmailsQuerySorter {
    fn from(kind: SearchEmailsQuerySorterKind) -> Self {
        (kind, None).into()
    }
}

/// The search emails sort kind.
///
/// Represents the property the sorter should sort emails from.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQuerySorterKind {
    /// Sort emails by message header `Date`.
    Date,

    /// Sort emails by envelope sender.
    From,

    /// Sort emails by envelope recipient.
    To,

    /// Sort emails by message header `Subject`.
    Subject,
}

/// The search emails sort order.
///
/// Defines in which order emails should be sorted.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQueryOrder {
    /// Sort emails by ascending order.
    #[default]
    Ascending,

    /// Sort emails by descending order.
    Descending,
}
