pub mod parser;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQuerySorterKind {
    Date,
    From,
    To,
    Subject,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQueryOrder {
    #[default]
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuerySorter(pub SearchEmailsQuerySorterKind, pub SearchEmailsQueryOrder);

impl SearchEmailsQuerySorter {
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

// impl SearchEmailsQuerySorter {
//     pub fn reverse(self) -> Self {
//         match self {
//             Self::Date(desc) => Self::Date(!desc),
//             Self::From(desc) => Self::From(!desc),
//             Self::To(desc) => Self::To(!desc),
//             Self::Subject(desc) => Self::Subject(!desc),
//         }
//     }
// }
