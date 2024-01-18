pub mod config;
pub mod dns;
pub mod http;

use email_address::EmailAddress;
use hyper::Uri;
use log::{debug, trace};
use std::str::FromStr;

use crate::{
    account::discover::config::{
        AuthenticationType, EmailProvider, EmailProviderProperty, SecurityType, Server,
        ServerProperty, ServerType,
    },
    Result,
};

use self::{config::AutoConfig, dns::Dns, http::Http};

pub async fn from_addr(addr: impl AsRef<str>) -> Result<AutoConfig> {
    let addr = EmailAddress::from_str(addr.as_ref())?;
    let http = Http::new();

    match from_isps(&http, &addr).await {
        Ok(config) => Ok(config),
        Err(err) => {
            debug!("{err}, falling back to DNS");
            from_dns(&http, &addr).await
        }
    }
}

async fn from_isps(http: &Http, addr: &EmailAddress) -> Result<AutoConfig> {
    match from_isp_main(&http, &addr).await {
        Ok(config) => Ok(config),
        Err(err) => {
            debug!("{err}, falling back to alternative ISP");
            match from_isp_alt(&http, addr.domain()).await {
                Ok(config) => Ok(config),
                Err(err) => {
                    debug!("{err}, falling back to ISPDB");
                    from_ispdb(&http, addr.domain()).await
                }
            }
        }
    }
}

async fn from_isp_main(http: &Http, addr: &EmailAddress) -> Result<AutoConfig> {
    let domain = addr.domain();

    let uri_str = format!("http://autoconfig.{domain}/mail/config-v1.1.xml?emailaddress={addr}");
    let uri = Uri::from_str(&uri_str).unwrap();

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from ISP at {uri_str}");
    trace!("{config:#?}");

    Ok(config)
}

async fn from_isp_alt(http: &Http, domain: &str) -> Result<AutoConfig> {
    let uri_str = format!("http://{domain}/.well-known/autoconfig/mail/config-v1.1.xml");
    let uri = Uri::from_str(&uri_str).unwrap();

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from ISP at {uri_str}");
    trace!("{config:#?}");

    Ok(config)
}

async fn from_ispdb(http: &Http, domain: &str) -> Result<AutoConfig> {
    let uri_str = format!("https://autoconfig.thunderbird.net/v1.1/{domain}");
    let uri = Uri::from_str(&uri_str).unwrap();

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from ISPDB at {uri_str}");
    trace!("{config:#?}");

    Ok(config)
}

async fn from_dns(http: &Http, addr: &EmailAddress) -> Result<AutoConfig> {
    let domain = addr.domain();
    let dns = Dns::new().await?;

    match from_dns_mx(http, &dns, domain).await {
        Ok(config) => Ok(config),
        Err(err) => {
            debug!("{err}, falling back to {domain} TXT records");
            match from_dns_txt(http, &dns, domain).await {
                Ok(config) => Ok(config),
                Err(err) => {
                    debug!("{err}, falling back to {domain} SRV records");
                    from_dns_srv(&dns, domain).await
                }
            }
        }
    }
}

async fn from_dns_mx(http: &Http, dns: &Dns, domain: &str) -> Result<AutoConfig> {
    let uri = dns.get_mailconf_mx_uri(domain).await?;

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from MX record");
    trace!("{config:#?}");

    Ok(config)
}

async fn from_dns_txt(http: &Http, dns: &Dns, domain: &str) -> Result<AutoConfig> {
    let uri = dns.get_mailconf_txt_uri(domain).await?;

    let config = http.get_config(uri).await?;
    debug!("successfully discovered config from TXT record");
    trace!("{config:#?}");

    Ok(config)
}

async fn from_dns_srv(dns: &Dns, domain: &str) -> Result<AutoConfig> {
    let mut config = AutoConfig {
        version: String::from("1.1"),
        email_provider: EmailProvider {
            id: domain.to_owned(),
            properties: Vec::new(),
        },
        oauth2: None,
    };

    if let Ok(record) = dns.get_imap_srv(domain).await {
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

    if let Ok(record) = dns.get_imaps_srv(domain).await {
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

    if let Ok(record) = dns.get_submission_srv(domain).await {
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
                        465 | _ => SecurityType::Tls,
                    }),
                    ServerProperty::Authentication(AuthenticationType::PasswordCleartext),
                ],
            }))
    }

    debug!("successfully discovered config from SRV record");
    trace!("{config:#?}");

    Ok(config)
}
