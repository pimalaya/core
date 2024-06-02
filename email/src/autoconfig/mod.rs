//! # Account discovery
//!
//! This module contains everything needed to discover account
//! configuration from a simple email address, heavily inspired by the
//! Thunderbird [Autoconfiguration] standard.
//!
//! *NOTE: only IMAP and SMTP configurations can be discovered by this
//! module.*
//!
//! Discovery performs actions in this order:
//!
//! - Check ISP databases for example.com
//!   - Check main ISP <autoconfig.example.com>
//!   - Check alt ISP <example.com/.well-known>
//!   - Check Thunderbird ISPDB <autoconfig.thunderbird.net/example.com>
//! - Check example.com DNS records
//!   - If example2.com found in example.com MX records
//!     - Check ISP databases for example2.com
//!     - Check for mailconf URI in example2.com TXT records
//!   - Check mailconf URI in example.com TXT records
//!   - Build autoconfig from imap and submission example.com SRV records
//!
//! [Autoconfiguration]: https://udn.realityripple.com/docs/Mozilla/Thunderbird/Autoconfiguration

pub mod config;
pub mod dns;
pub mod http;

use std::{result, str::FromStr};

use email_address::EmailAddress;
use futures::{future::select_ok, FutureExt};
use hyper::{StatusCode, Uri};
use thiserror::Error;

use self::{
    config::{AutoConfig, EmailProvider},
    dns::DnsClient,
    http::HttpClient,
};
use crate::{debug, trace};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find any MX record at {0}")]
    GetMxRecordNotFoundError(String),
    #[error("cannot find any mailconf TXT record at {0}")]
    GetMailconfTxtRecordNotFoundError(String),
    #[error("cannot find any SRV record at {0}")]
    GetSrvRecordNotFoundError(String),
    #[error("cannot do txt lookup: {0}")]
    LookUpTxtError(#[source] hickory_resolver::error::ResolveError),
    #[error("cannot do mx lookup: {0}")]
    LookUpMxError(#[source] hickory_resolver::error::ResolveError),
    #[error("cannot do srv lookup: {0}")]
    LookUpSrvError(#[source] hickory_resolver::error::ResolveError),
    #[error("cannot get autoconfig from {0}: {1}")]
    GetAutoConfigError(Uri, StatusCode),
    #[error("cannot do a get request for autoconfig from {0}: {1}")]
    GetConnectionAutoConfigError(Uri, #[source] hyper::Error),
    #[error("cannot get the body of response for autoconfig from {0}: {1}")]
    ToBytesAutoConfigError(Uri, #[source] hyper::Error),
    #[error("cannot decode the body of response for autoconfig from {0}: {1}")]
    SerdeXmlFailedForAutoConfig(Uri, #[source] serde_xml_rs::Error),
    #[error("cannot parse email {0}: {1}")]
    ParsingEmailAddress(String, #[source] email_address::Error),
}

/// Discover configuration associated to a given email address using
/// ISP locations then DNS, as described in the Mozilla [wiki].
///
/// [wiki]: https://wiki.mozilla.org/Thunderbird:Autoconfiguration#Implementation
pub async fn from_addr(addr: impl AsRef<str>) -> Result<AutoConfig> {
    let addr = EmailAddress::from_str(addr.as_ref())
        .map_err(|e| Error::ParsingEmailAddress(addr.as_ref().to_string(), e))?;
    let http = HttpClient::new();

    match from_isps(&http, &addr).await {
        Ok(config) => Ok(config),
        Err(_err) => {
            trace!("{_err}");
            debug!("ISP discovery failed for {addr}, falling back to DNS");
            from_dns(&http, &addr).await
        }
    }
}

/// Discover configuration associated to a given email address using
/// different ISP locations, as described in the Mozilla [wiki].
///
/// Inspect first main ISP locations, then inspect alternative ISP
/// locations.
///
/// [wiki]: https://wiki.mozilla.org/Thunderbird:Autoconfiguration#Implementation
async fn from_isps(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    let from_main_isps = [
        from_plain_main_isp(http, addr).boxed(),
        from_secure_main_isp(http, addr).boxed(),
    ];

    match select_ok(from_main_isps).await {
        Ok((config, _)) => Ok(config),
        Err(_err) => {
            trace!("{_err}");
            debug!("main ISP discovery failed for {addr}, falling back to alternative ISP");

            let from_alt_isps = [
                from_plain_alt_isp(http, addr).boxed(),
                from_secure_alt_isp(http, addr).boxed(),
            ];

            match select_ok(from_alt_isps).await {
                Ok((config, _)) => Ok(config),
                Err(_err) => {
                    trace!("{_err}");
                    debug!("alternative ISP discovery failed for {addr}, falling back to ISPDB");
                    from_ispdb(http, addr).await
                }
            }
        }
    }
}

/// Discover configuration associated to a given email address using
/// plain main ISP location (http).
async fn from_plain_main_isp(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    from_main_isp(http, "http", addr).await
}

/// Discover configuration associated to a given email address using
/// secure main ISP location (https).
async fn from_secure_main_isp(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    from_main_isp(http, "https", addr).await
}

/// Discover configuration associated to a given email address using
/// main ISP location.
async fn from_main_isp(http: &HttpClient, scheme: &str, addr: &EmailAddress) -> Result<AutoConfig> {
    let domain = addr.domain().trim_matches('.');
    let uri_str =
        format!("{scheme}://autoconfig.{domain}/mail/config-v1.1.xml?emailaddress={addr}");
    let uri = Uri::from_str(&uri_str).unwrap();

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from ISP at {uri_str}");
    trace!("{config:#?}");

    Ok(config)
}

/// Discover configuration associated to a given email address using
/// plain alternative ISP location (http).
async fn from_plain_alt_isp(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    from_alt_isp(http, "http", addr).await
}

/// Discover configuration associated to a given email address using
/// secure alternative ISP location (https).
async fn from_secure_alt_isp(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    from_alt_isp(http, "https", addr).await
}

/// Discover configuration associated to a given email address using
/// alternative ISP location.
async fn from_alt_isp(http: &HttpClient, scheme: &str, addr: &EmailAddress) -> Result<AutoConfig> {
    let domain = addr.domain().trim_matches('.');
    let uri_str = format!("{scheme}://{domain}/.well-known/autoconfig/mail/config-v1.1.xml");
    let uri = Uri::from_str(&uri_str).unwrap();

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from ISP at {uri_str}");
    trace!("{config:#?}");

    Ok(config)
}

/// Discover configuration associated to a given email address using
/// Thunderbird ISPDB.
async fn from_ispdb(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    let domain = addr.domain().trim_matches('.');
    let uri_str = format!("https://autoconfig.thunderbird.net/v1.1/{domain}");
    let uri = Uri::from_str(&uri_str).unwrap();

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from ISPDB at {uri_str}");
    trace!("{config:#?}");

    Ok(config)
}

/// Discover configuration associated to a given email address using
/// different DNS records.
///
/// Inspect first MX records, then TXT records, and finally SRV
/// records.
async fn from_dns(http: &HttpClient, addr: &EmailAddress) -> Result<AutoConfig> {
    let domain = addr.domain().trim_matches('.');
    let dns = DnsClient::new();

    match from_dns_mx(http, &dns, addr).await {
        Ok(config) => Ok(config),
        Err(_err) => {
            trace!("{_err}");
            debug!("MX discovery failed for {addr}, falling back to TXT records");
            match from_dns_txt(http, &dns, domain).await {
                Ok(config) => Ok(config),
                Err(_err) => {
                    trace!("{_err}");
                    debug!("TXT discovery failed for {addr}, falling back to SRV records");
                    from_dns_srv(&dns, domain).await
                }
            }
        }
    }
}

/// Discover configuration associated to a given email address using
/// MX DNS records.
async fn from_dns_mx(
    http: &HttpClient,
    dns: &DnsClient,
    addr: &EmailAddress,
) -> Result<AutoConfig> {
    let local_part = addr.local_part();
    let domain = dns.get_mx_domain(addr.domain()).await?;
    let domain = domain.trim_matches('.');
    let addr = EmailAddress::from_str(&format!("{local_part}@{domain}")).unwrap();

    match from_isps(http, &addr).await {
        Ok(config) => Ok(config),
        Err(_err) => {
            trace!("{err}");
            debug!("ISP discovery failed for {domain}, falling back to TXT records");
            from_dns_txt(http, dns, domain).await
        }
    }
}

/// Discover configuration associated to a given email address using
/// TXT DNS records.
async fn from_dns_txt(http: &HttpClient, dns: &DnsClient, domain: &str) -> Result<AutoConfig> {
    let uri = dns.get_mailconf_txt_uri(domain).await?;

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from {domain} TXT record");
    trace!("{config:#?}");

    Ok(config)
}

/// Discover configuration associated to a given email address using
/// SRV DNS records.
async fn from_dns_srv(
    #[allow(unused_variables)] dns: &DnsClient,
    domain: &str,
) -> Result<AutoConfig> {
    #[allow(unused_mut)]
    let mut config = AutoConfig {
        version: String::from("1.1"),
        email_provider: EmailProvider {
            id: domain.to_owned(),
            properties: Vec::new(),
        },
        oauth2: None,
    };

    #[cfg(feature = "imap")]
    if let Ok(record) = dns.get_imap_srv(domain).await {
        use self::config::{
            AuthenticationType, EmailProviderProperty, SecurityType, Server, ServerProperty,
            ServerType,
        };

        config
            .email_provider
            .properties
            .push(EmailProviderProperty::IncomingServer(Server {
                r#type: ServerType::Imap,
                properties: vec![
                    ServerProperty::Hostname(record.target().to_string()),
                    ServerProperty::Port(record.port()),
                    ServerProperty::SocketType(SecurityType::Starttls),
                    ServerProperty::Authentication(AuthenticationType::PasswordCleartext),
                ],
            }))
    }

    #[cfg(feature = "imap")]
    if let Ok(record) = dns.get_imaps_srv(domain).await {
        use self::config::{
            AuthenticationType, EmailProviderProperty, SecurityType, Server, ServerProperty,
            ServerType,
        };

        config
            .email_provider
            .properties
            .push(EmailProviderProperty::IncomingServer(Server {
                r#type: ServerType::Imap,
                properties: vec![
                    ServerProperty::Hostname(record.target().to_string()),
                    ServerProperty::Port(record.port()),
                    ServerProperty::SocketType(SecurityType::Tls),
                    ServerProperty::Authentication(AuthenticationType::PasswordCleartext),
                ],
            }))
    }

    #[cfg(feature = "smtp")]
    if let Ok(record) = dns.get_submission_srv(domain).await {
        use self::config::{
            AuthenticationType, EmailProviderProperty, SecurityType, Server, ServerProperty,
            ServerType,
        };

        config
            .email_provider
            .properties
            .push(EmailProviderProperty::OutgoingServer(Server {
                r#type: ServerType::Smtp,
                properties: vec![
                    ServerProperty::Hostname(record.target().to_string()),
                    ServerProperty::Port(record.port()),
                    ServerProperty::SocketType(match record.port() {
                        25 => SecurityType::Plain,
                        587 => SecurityType::Starttls,
                        _ => SecurityType::Tls, // including 456
                    }),
                    ServerProperty::Authentication(AuthenticationType::PasswordCleartext),
                ],
            }))
    }

    debug!("successfully discovered config from {domain} SRV record");
    trace!("{config:#?}");

    Ok(config)
}
