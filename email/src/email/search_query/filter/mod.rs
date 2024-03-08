pub mod parser;

use chrono::NaiveDate;

use crate::flag::Flag;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQueryFilter {
    And(Box<SearchEmailsQueryFilter>, Box<SearchEmailsQueryFilter>),
    Or(Box<SearchEmailsQueryFilter>, Box<SearchEmailsQueryFilter>),
    Not(Box<SearchEmailsQueryFilter>),
    Date(NaiveDate),
    BeforeDate(NaiveDate),
    AfterDate(NaiveDate),
    From(String),
    To(String),
    Subject(String),
    Body(String),
    Flag(Flag),
}
