pub mod config;
pub mod dns;
pub mod http;

use anyhow::bail;
use email_address::EmailAddress;
use futures::{future::select_ok, FutureExt};
use log::debug;
use std::str::FromStr;

use crate::Result;

use self::{config::AutoConfig, dns::Dns, http::Http};

/// Given an email providers domain, try to connect to autoconfig servers for that provider and return the config.
pub async fn from_domain<D: AsRef<str>>(domain: D) -> Result<AutoConfig> {
    let mut errors: Vec<_> = Vec::new();

    let http = Http::new();
    let dns = Dns::new().await?;

    let mut futures = Vec::new();

    let mut urls = vec![
        // Try connect to connect with the users mail server directly
        format!("http://autoconfig.{}/mail/config-v1.1.xml", domain.as_ref()),
        // The fallback url
        format!(
            "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
            domain.as_ref()
        ),
        // If the previous two methods did not work then the email server provider has not setup Thunderbird autoconfig, so we ask Mozilla for their config.
        format!(
            "https://autoconfig.thunderbird.net/v1.1/{}",
            domain.as_ref()
        ),
    ];

    match dns.get_first_mailconf_mx_uri(domain.as_ref()).await {
        Ok(uri) => urls.push(uri.to_string()),
        Err(err) => {
            debug!("skipping MX record config discovery: {err}");
        }
    };

    urls.sort();
    urls.dedup();

    for url in urls {
        let future = http.get_config(url);
        futures.push(future.boxed());
    }

    let result = select_ok(futures).await;

    match result {
        Ok((config, _remaining)) => return Ok(config),
        Err(error) => errors.push(error),
    }

    bail!("auto config not found")
}

/// Given an email address, try to connect to the email providers autoconfig servers and return the config that was found, if one was found.
pub async fn from_addr(email: impl AsRef<str>) -> Result<AutoConfig> {
    let email = EmailAddress::from_str(email.as_ref())?;
    from_domain(email.domain()).await
}
