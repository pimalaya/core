//! # Search emails query parser
//!
//! This module contains parsers needed to parse a full search emails
//! query, and exposes a [`query`] parser. Parsing is based on the
//! great lib [chumsky].

use chumsky::prelude::*;

use super::{
    filter::parser::filters,
    sorter::parser::{order_by, sorters},
    SearchEmailsQuery,
};

pub(crate) type ParserError<'a> = extra::Err<Rich<'a, char>>;

pub(crate) fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    choice((
        filters()
            .then_ignore(
                just(' ')
                    .labelled("space before `order by`")
                    .repeated()
                    .at_least(1),
            )
            .then_ignore(order_by())
            .then(sorters())
            .map(|(filters, sorters)| SearchEmailsQuery {
                filters: Some(filters),
                sorters: Some(sorters),
            }),
        filters().map(|filters| SearchEmailsQuery {
            filters: Some(filters),
            sorters: None,
        }),
        sorters().map(|sorters| SearchEmailsQuery {
            filters: None,
            sorters: Some(sorters),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use crate::search_query::{
        filter::SearchEmailsQueryFilter,
        sorter::{SearchEmailsQueryOrder, SearchEmailsQuerySorter},
        SearchEmailsQuery,
    };

    #[test]
    fn filters_only() {
        assert_eq!(
            super::query().parse("from f and to t").into_result(),
            Ok(SearchEmailsQuery {
                filters: Some(SearchEmailsQueryFilter::And(
                    Box::new(SearchEmailsQueryFilter::From("f".into())),
                    Box::new(SearchEmailsQueryFilter::Subject("s".into()))
                )),
                sorters: None,
            }),
        );
    }

    #[test]
    fn sorters_only() {
        assert_eq!(
            super::query().parse("from").into_result(),
            Ok(SearchEmailsQuery {
                filters: None,
                sorters: Some(vec![SearchEmailsQuerySorter::From(
                    SearchEmailsQueryOrder::Ascending
                ),])
            }),
        );

        assert_eq!(
            super::query().parse("from subject:desc").into_result(),
            Ok(SearchEmailsQuery {
                filters: None,
                sorters: Some(vec![
                    SearchEmailsQuerySorter::From(SearchEmailsQueryOrder::Ascending),
                    SearchEmailsQuerySorter::Subject(SearchEmailsQueryOrder::Descending)
                ])
            }),
        );
    }

    #[test]
    fn query() {
        assert_eq!(
            super::query()
                .parse("from f and to t order by from to:desc")
                .into_result(),
            Ok(SearchEmailsQuery {
                filters: Some(SearchEmailsQueryFilter::And(
                    Box::new(SearchEmailsQueryFilter::From("f".into())),
                    Box::new(SearchEmailsQueryFilter::Subject("s".into()))
                )),
                sorters: Some(vec![
                    SearchEmailsQuerySorter::From(SearchEmailsQueryOrder::Ascending),
                    SearchEmailsQuerySorter::To(SearchEmailsQueryOrder::Descending)
                ])
            }),
        );
    }
}
