//! # Search emails sort query
//!
//! This module exposes [`SearchEmailsSortQuery`], a structure that
//! helps you to sort emails.
//!
//! The search emails sort query can be parsed from a string, see the
//! [`parser::query`] module for more details.

pub mod parser;

/// The search emails sort query.
///
/// The sort query is just a list of [`SearchEmailsSorter`].
pub type SearchEmailsSortQuery = Vec<SearchEmailsSorter>;

/// The search emails sorter.
///
/// The sorter is composed of a kind (date, from, to, subject) and an
/// order (ascending, descending).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsSorter(
    /// The search emails sorter kind.
    pub SearchEmailsSorterKind,
    /// The search emails sorter order.
    pub SearchEmailsSorterOrder,
);

impl SearchEmailsSorter {
    /// Create a new search emails sorter from a kind and an order.
    pub fn new(kind: SearchEmailsSorterKind, order: SearchEmailsSorterOrder) -> Self {
        Self(kind, order)
    }
}

impl From<(SearchEmailsSorterKind, SearchEmailsSorterOrder)> for SearchEmailsSorter {
    fn from((kind, order): (SearchEmailsSorterKind, SearchEmailsSorterOrder)) -> Self {
        SearchEmailsSorter::new(kind, order)
    }
}

impl From<(SearchEmailsSorterKind, Option<SearchEmailsSorterOrder>)> for SearchEmailsSorter {
    fn from((kind, order): (SearchEmailsSorterKind, Option<SearchEmailsSorterOrder>)) -> Self {
        (kind, order.unwrap_or_default()).into()
    }
}

impl From<SearchEmailsSorterKind> for SearchEmailsSorter {
    fn from(kind: SearchEmailsSorterKind) -> Self {
        (kind, None).into()
    }
}

/// The search emails sorter kind.
///
/// Represents the property the sorter should sort emails from.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsSorterKind {
    /// Sort emails by message header `Date`.
    Date,

    /// Sort emails by envelope sender.
    From,

    /// Sort emails by envelope recipient.
    To,

    /// Sort emails by message header `Subject`.
    Subject,
}

/// The search emails sorter order.
///
/// Defines in which order emails should be sorted.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsSorterOrder {
    /// Sort emails by ascending order.
    #[default]
    Ascending,

    /// Sort emails by descending order.
    Descending,
}
