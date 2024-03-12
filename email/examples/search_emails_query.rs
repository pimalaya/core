use chrono::NaiveDate;
use email::{
    account::config::{passwd::PasswdConfig, AccountConfig},
    backend::{Backend, BackendBuilder},
    envelope::list::{ListEnvelopes, ListEnvelopesOptions},
    imap::{
        config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
        ImapContextBuilder, ImapContextSync,
    },
    search_query::{
        filter::SearchEmailsFilterQuery,
        sort::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
        SearchEmailsQuery,
    },
};
use email_testing_server::with_email_testing_server;
use secret::Secret;
use std::sync::Arc;

#[tokio::main]
pub async fn main() {
    with_email_testing_server(|ports| async move {
        let account_config = Arc::new(AccountConfig::default());

        let imap_config = Arc::new(ImapConfig {
            host: "localhost".into(),
            port: ports.imap,
            encryption: Some(ImapEncryptionKind::None),
            login: "alice".into(),
            auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_raw("password"))),
            ..Default::default()
        });
        let imap_ctx = ImapContextBuilder::new(account_config.clone(), imap_config.clone());
        let imap = BackendBuilder::new(account_config, imap_ctx)
            .build::<Backend<ImapContextSync>>()
            .await
            .unwrap();

        let query = SearchEmailsQuery {
            filter: Some(SearchEmailsFilterQuery::And(
                Box::new(SearchEmailsFilterQuery::Subject(String::from("foo"))),
                Box::new(SearchEmailsFilterQuery::AfterDate(
                    NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                )),
            )),
            sort: Some(vec![
                SearchEmailsSorter::new(
                    SearchEmailsSorterKind::Date,
                    SearchEmailsSorterOrder::Descending,
                ),
                SearchEmailsSorter::new(
                    SearchEmailsSorterKind::Subject,
                    SearchEmailsSorterOrder::Ascending,
                ),
            ]),
        };

        let envelopes = imap
            .list_envelopes(
                "INBOX",
                ListEnvelopesOptions {
                    page: 1,
                    page_size: 10,
                    query: Some(query),
                },
            )
            .await
            .unwrap();

        assert_eq!(envelopes.len(), 0)
    })
    .await
}