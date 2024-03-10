//! # Search emails filter query
//!
//! This module exposes [`SearchEmailsFilterQuery`], a structure that
//! helps you to filter emails.
//!
//! The search emails filter query can be parsed from a string, see
//! the [`parser::query`] module for more details.

pub mod parser;

use chrono::NaiveDate;

use crate::flag::Flag;

/// The search emails filter query.
///
/// The filter query is composed of 3 operators (and, or, not) and 9
/// conditions (date, before date, after date, from, to, subject, body
/// and flag).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsFilterQuery {
    /// Filter emails that match the 2 given conditions.
    And(Box<SearchEmailsFilterQuery>, Box<SearchEmailsFilterQuery>),

    /// Filter emails that match one of the 2 given conditions.
    Or(Box<SearchEmailsFilterQuery>, Box<SearchEmailsFilterQuery>),

    /// Filter emails that does not match the given condition.
    Not(Box<SearchEmailsFilterQuery>),

    /// Filter emails where the `Date` header of the message matches
    /// the given date.
    ///
    /// Only the year, the month and the day are taken into
    /// consideration.
    Date(NaiveDate),

    /// Filter emails where the `Date` header of the message is
    /// strictly less than the given date.
    ///
    /// For example, for a given date `2024-01-01`, it will match
    /// messages with a date starting from `2023-12-31` and
    /// below. Only the year, the month and the day are taken into
    /// consideration.
    BeforeDate(NaiveDate),

    /// Filter emails where the `Date` header of the message is
    /// strictly greater than the given date.
    ///
    /// For example, for a given date `2024-01-01`, it will match
    /// messages with a date starting from `2024-01-02` and
    /// above. Only the year, the month and the day are taken into
    /// consideration.
    AfterDate(NaiveDate),

    /// Filter emails where the `From` header of the message contains
    /// the given pattern.
    From(String),

    /// Filter emails where the `To` header of the message contains
    /// the given pattern.
    To(String),

    /// Filter emails where the `Subject` header of the message
    /// contains the given pattern.
    Subject(String),

    /// Filter emails where one of the text body of the message
    /// contains the given pattern.
    Body(String),

    /// Filter emails where the given flag is included in the email
    /// envelope flags.
    Flag(Flag),
}
