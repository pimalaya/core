pub mod parser;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQuerySorter {
    Date(SearchEmailsQueryOrder),
    From(SearchEmailsQueryOrder),
    To(SearchEmailsQueryOrder),
    Subject(SearchEmailsQueryOrder),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQueryOrder {
    #[default]
    Ascending,
    Descending,
}
