//! # Account config discovery
//!
//! This module contains everything needed to discover account
//! configuration from a simple email address, based on the
//! Thunderbird [Autoconfiguration] standard.
//!
//! *NOTE: only IMAP and SMTP configurations can be discovered by this
//! module.*
//!
//! [Autoconfiguration]: https://udn.realityripple.com/docs/Mozilla/Thunderbird/Autoconfiguration#Mechanisms

use bytes::Bytes;
use hickory_resolver::{
    proto::rr::rdata::{MX, SRV},
    TokioAsyncResolver,
};
use hyper::Uri;
use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{cmp::Ordering, ops::Deref};
use thiserror::Error;

use crate::Result;

static TXT_RECORD_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^mailconf=(https://\S+)$").unwrap());

/// Errors related to PGP encryption.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find MX record for domain {0}")]
    GetMxRecordNotFoundError(String),
    #[error("cannot find mailconf TXT record for domain {0}")]
    GetMailconfTxtRecordNotFoundError(String),
}

pub struct Dns {
    resolver: TokioAsyncResolver,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SortableMX(MX);

impl Deref for SortableMX {
    type Target = MX;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for SortableMX {
    fn cmp(&self, other: &Self) -> Ordering {
        other.preference().cmp(&self.preference())
    }
}

impl PartialOrd for SortableMX {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl SortableMX {
    pub fn new(mx: MX) -> Self {
        Self(mx)
    }
}

struct WeightedSrvRecord {
    record: SRV,
    weight: u16,
    priority: u16,
}

impl WeightedSrvRecord {
    fn new(record: SRV) -> Self {
        Self {
            weight: record.weight(),
            priority: record.priority(),
            record,
        }
    }
}

// Compare WeightedSrvRecord by priority and weight
impl Ord for WeightedSrvRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        // Sort by priority in ascending order
        let priority_cmp = self.priority.cmp(&other.priority);

        if priority_cmp == Ordering::Equal {
            // Sort by weight in descending order
            other.weight.cmp(&self.weight)
        } else {
            priority_cmp
        }
    }
}

// Implement PartialOrd for WeightedSrvRecord
impl PartialOrd for WeightedSrvRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Implement Eq for WeightedSrvRecord
impl Eq for WeightedSrvRecord {}

// Implement PartialEq for WeightedSrvRecord
impl PartialEq for WeightedSrvRecord {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.weight == other.weight
    }
}

impl Dns {
    pub async fn new() -> Result<Self> {
        let resolver = TokioAsyncResolver::tokio_from_system_conf()?;
        let dns = Self { resolver };
        Ok(dns)
    }

    async fn get_lowest_mx_exchange(&self, domain: &str) -> Result<String> {
        let mut records: Vec<SortableMX> = self
            .resolver
            .mx_lookup(domain)
            .await?
            .into_iter()
            .map(|record| {
                debug!("{domain}: discovering MX record: {record}");
                SortableMX::new(record)
            })
            .collect();

        records.sort();

        let record = records
            .pop()
            .ok_or_else(|| Error::GetMxRecordNotFoundError(domain.to_owned()))?;

        let exchange = record.exchange().trim_to(2).to_string();

        debug!("{domain}: use MX domain {exchange}");

        Ok(exchange)
    }

    async fn get_first_mailconf_txt_uri(&self, domain: &str) -> Result<Uri> {
        let mut records = self
            .resolver
            .txt_lookup(domain)
            .await?
            .into_iter()
            .map(|record| {
                debug!("{domain}: discovering TXT record: {record}");
                record.to_string()
            });

        let uri = records
            .find_map(|record| {
                TXT_RECORD_REGEX
                    .captures(&record)
                    .and_then(|captures| captures.get(1))
                    .and_then(|capture| capture.as_str().parse::<Uri>().ok())
            })
            .ok_or_else(|| Error::GetMailconfTxtRecordNotFoundError(domain.to_owned()))?;

        debug!("{domain}: use mailconf URI {uri}");

        Ok(uri)
    }

    pub async fn get_first_mailconf_mx_uri(&self, domain: &str) -> Result<Uri> {
        let domain = self.get_lowest_mx_exchange(domain).await?;
        self.get_first_mailconf_txt_uri(&domain).await
    }

    // async fn _srv_lookup(&self, query: impl ToString) -> Result<(String, u16)> {
    //     let records = self.resolver.srv_lookup(query.to_string()).await?;

    //     let mut weighted_records: Vec<_> =
    //         records.into_iter().map(WeightedSrvRecord::new).collect();

    //     weighted_records.sort();

    //     if let Some(record) = weighted_records.first() {
    //         return Ok((record.record.target().to_string(), record.record.port()));
    //     }

    //     bail!("cannot not find domains from the SRV query")
    // }

    // /// Lookup basic dns settings to find mail servers according to https://datatracker.ietf.org/doc/html/rfc6186
    // pub async fn srv_lookup<D: AsRef<str>>(&self, domain: D) -> Vec<Server> {
    //     let server_types = vec![Pop, Imap];

    //     let mut queries = Vec::new();

    //     for server_type in server_types {
    //         let plain = DnsQuery::new(domain.as_ref(), false, server_type);
    //         let secure = DnsQuery::new(domain.as_ref(), true, server_type);

    //         queries.push(plain);
    //         queries.push(secure);
    //     }

    //     queries.push(DnsQuery::new(domain.as_ref(), true, Smtp));

    //     let mut servers = Vec::new();

    //     for query in queries {
    //         let result = self._srv_lookup(&query).await;

    //         match result {
    //             Ok((domain, port)) => {
    //                 if &domain != "." {
    //                     let server = Server::new(port, domain, query.into());
    //                     servers.push(server);
    //                 }
    //             }
    //             Err(error) => {
    //                 warn!("SRV lookup failed for {}: {}", query, error)
    //             }
    //         }
    //     }

    //     servers
    // }

    async fn get_txt(&self, name: impl AsRef<str>) -> Result<Vec<Bytes>> {
        let lookup_results = self.resolver.txt_lookup(name.as_ref()).await?;
        let mut records = Vec::new();

        for txt in lookup_results {
            println!("txt: {:?}", txt.to_string());
            let mut bytes: Vec<Bytes> = txt
                .txt_data()
                .iter()
                .map(|data| data.to_vec().into())
                .collect();

            if bytes.first().is_some() {
                let record = bytes.remove(0);
                records.push(record);
            }
        }

        Ok(records)
    }

    pub async fn get_url_from_txt(&self, name: impl AsRef<str>) -> Result<Vec<String>> {
        let records = self.get_txt(name).await?;

        let mut urls = Vec::new();

        for record in records {
            if let Some(record_str) = std::str::from_utf8(&record).ok() {
                if let Some(captured) = TXT_RECORD_REGEX.captures(record_str) {
                    if let Some(r#match) = captured.get(1) {
                        let url = r#match.as_str();

                        println!("url: {:?}", url);

                        if let Some(url_parsed) = url.parse::<Uri>().ok() {
                            if let Some("https") = url_parsed.scheme_str() {
                                urls.push(url.to_string())
                            }
                        }
                    }
                }
            }
        }

        Ok(urls)
    }
}
