pub mod parser;

use chrono::{DateTime, Local};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQueryFilter {
    And(Box<SearchEmailsQueryFilter>, Box<SearchEmailsQueryFilter>),
    Or(Box<SearchEmailsQueryFilter>, Box<SearchEmailsQueryFilter>),
    Not(Box<SearchEmailsQueryFilter>),
    Date(DateTime<Local>),
    BeforeDate(DateTime<Local>),
    AfterDate(DateTime<Local>),
    From(String),
    To(String),
    Subject(String),
    Body(String),
    Keyword(String),
}
