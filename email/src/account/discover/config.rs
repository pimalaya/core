use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// A config that follows Mozilla's specification: https://wiki.mozilla.org/Thunderbird:Autoconfiguration:ConfigFileFormat
pub struct AutoConfig {
    version: String,
    email_provider: EmailProvider,
    #[serde(rename = "oAuth2")]
    oauth2: Option<OAuth2Config>,
}

impl AutoConfig {
    /// The config version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Information about the email provider for the given email address, e.g. Google or Microsoft
    pub fn email_provider(&self) -> &EmailProvider {
        &self.email_provider
    }

    /// If the provider supports oAuth2, it SHOULD be specified here, but some providers don't.
    pub fn oauth2(&self) -> Option<&OAuth2Config> {
        self.oauth2.as_ref()
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Config {
    issuer: String,
    scope: String,
    #[serde(rename = "authURL")]
    auth_url: String,
    #[serde(rename = "tokenURL")]
    token_url: String,
}

impl OAuth2Config {
    /// The implementer of the oAuth2 protocol for this email provider, which is usually the email provider itself.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// The scopes needed from the oAuth2 API to be able to login using an oAuth2 generated token.
    pub fn scope(&self) -> Vec<&str> {
        self.scope.split(' ').collect()
    }

    /// The url where the initial oAuth2 login takes place.
    pub fn auth_url(&self) -> &str {
        &self.auth_url
    }

    /// The url used to refresh the oAuth2 token.
    pub fn token_url(&self) -> &str {
        &self.token_url
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct EmailProvider {
    id: String,
    #[serde(rename = "$value")]
    properties: Vec<EmailProviderProperty>,
}

impl EmailProvider {
    /// Just an array containing all of the email providers properties, usefull if you want to get multiple properties in 1 for loop.
    pub fn properties(&self) -> &Vec<EmailProviderProperty> {
        &self.properties
    }

    /// The email providers unique id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The domain name that the email provider uses in their email addresses.
    pub fn domain(&self) -> Vec<&str> {
        let mut domains: Vec<&str> = Vec::new();

        for property in &self.properties {
            match property {
                EmailProviderProperty::Domain(domain) => domains.push(domain),
                _ => {}
            }
        }

        domains
    }

    /// The email providers display name. e.g. Google Mail
    pub fn display_name(&self) -> Option<&str> {
        for property in &self.properties {
            match property {
                EmailProviderProperty::DisplayName(display_name) => return Some(display_name),
                _ => {}
            }
        }

        None
    }

    /// The email providers short display name. e.g. GMail
    pub fn display_short_name(&self) -> Option<&str> {
        for property in &self.properties {
            match property {
                EmailProviderProperty::DisplayShortName(short_name) => return Some(short_name),
                _ => {}
            }
        }

        None
    }

    /// An array containing info about all of an email providers incoming mail servers
    pub fn incoming_servers(&self) -> Vec<&Server> {
        let mut servers: Vec<&Server> = Vec::new();

        for property in &self.properties {
            match property {
                EmailProviderProperty::IncomingServer(server) => servers.push(server),
                _ => {}
            }
        }

        servers
    }

    /// An array containing info about all of an email providers outgoing mail servers
    pub fn outgoing_servers(&self) -> Vec<&Server> {
        let mut servers: Vec<&Server> = Vec::new();

        for property in &self.properties {
            match property {
                EmailProviderProperty::OutgoingServer(server) => servers.push(server),
                _ => {}
            }
        }

        servers
    }

    /// An array containing info about all of an email providers mail servers
    pub fn servers(&self) -> Vec<&Server> {
        let mut servers: Vec<&Server> = Vec::new();

        for property in &self.properties {
            match property {
                EmailProviderProperty::IncomingServer(server) => servers.push(server),
                EmailProviderProperty::OutgoingServer(server) => servers.push(server),
                _ => {}
            }
        }

        servers
    }

    /// Documentation on how to setup the email client, provided by the email provider.
    pub fn documentation(&self) -> Option<&Documentation> {
        for property in &self.properties {
            match property {
                EmailProviderProperty::Documentation(documentation) => return Some(documentation),
                _ => {}
            }
        }

        None
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EmailProviderProperty {
    Domain(String),
    DisplayName(String),
    DisplayShortName(String),
    IncomingServer(Server),
    OutgoingServer(Server),
    Documentation(Documentation),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Server {
    r#type: ServerType,
    #[serde(rename = "$value")]
    properties: Vec<ServerProperty>,
}

impl Server {
    /// Just an array containing all of a mail servers properties, usefull if you want to get multiple properties in 1 for loop.
    pub fn properties(&self) -> &Vec<ServerProperty> {
        &self.properties
    }

    /// What type of mail server this server is.
    pub fn server_type(&self) -> &ServerType {
        &self.r#type
    }

    /// The mail servers domain/ip
    pub fn hostname(&self) -> Option<&str> {
        for property in &self.properties {
            match property {
                ServerProperty::Hostname(hostname) => return Some(hostname),
                _ => {}
            }
        }

        None
    }

    /// The mail servers port
    pub fn port(&self) -> Option<&u16> {
        for property in &self.properties {
            match property {
                ServerProperty::Port(port) => return Some(port),
                _ => {}
            }
        }

        None
    }

    /// The kind of security the mail server prefers
    pub fn security_type(&self) -> Option<&SecurityType> {
        for property in &self.properties {
            match property {
                ServerProperty::SocketType(socket_type) => return Some(socket_type),
                _ => {}
            }
        }

        None
    }

    /// The kind of authentication is needed to login to this mail server
    pub fn authentication_type(&self) -> Vec<&AuthenticationType> {
        let mut types: Vec<&AuthenticationType> = Vec::new();

        for property in &self.properties {
            match property {
                ServerProperty::Authentication(authentication_type) => {
                    types.push(authentication_type)
                }
                _ => {}
            }
        }

        types
    }

    /// The users username
    pub fn username(&self) -> Option<&str> {
        for property in &self.properties {
            match property {
                ServerProperty::Username(username) => return Some(&username),
                _ => {}
            }
        }

        None
    }

    /// The users password
    pub fn password(&self) -> Option<&str> {
        for property in &self.properties {
            match property {
                ServerProperty::Password(password) => return Some(&password),
                _ => {}
            }
        }

        None
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ServerProperty {
    Hostname(String),
    Port(u16),
    #[serde(rename = "socketType")]
    SocketType(SecurityType),
    Authentication(AuthenticationType),
    OwaURL(String),
    EwsURL(String),
    UseGlobalPreferredServer(bool),
    Pop3(Pop3Config),
    Username(String),
    Password(String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum SecurityType {
    #[serde(rename = "plain")]
    Plain,
    #[serde(rename = "STARTTLS")]
    Starttls,
    #[serde(rename = "SSL")]
    Tls,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ServerType {
    Exchange,
    Imap,
    Pop3,
    Smtp,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum AuthenticationType {
    #[serde(rename = "password-cleartext")]
    PasswordCleartext,
    #[serde(rename = "password-encrypted")]
    PasswordEncrypted,
    #[serde(rename = "NTLM")]
    Ntlm,
    #[serde(rename = "GSAPI")]
    GsApi,
    #[serde(rename = "client-IP-address")]
    ClientIPAddress,
    #[serde(rename = "TLS-client-cert")]
    TlsClientCert,
    OAuth2,
    #[serde(rename = "None")]
    None,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pop3Config {
    leave_messages_on_server: bool,
    download_on_biff: Option<bool>,
    days_to_leave_messages_on_server: Option<u64>,
    check_interval: Option<CheckInterval>,
}

impl Pop3Config {
    /// If the server should leave all of the read messages on the server after the client quits the connection.
    pub fn leave_messages_on_server(&self) -> &bool {
        &self.leave_messages_on_server
    }

    pub fn download_on_biff(&self) -> Option<&bool> {
        self.download_on_biff.as_ref()
    }

    /// How long the Pop messages will be stored on the server.
    pub fn time_to_leave_messages_on_server(&self) -> Option<Duration> {
        match self.days_to_leave_messages_on_server.as_ref() {
            Some(days) => Some(Duration::from_secs(days * 24 * 60 * 60)),
            None => None,
        }
    }

    /// The interval in which the server will allow a check for new messages. Not supported on all servers.
    pub fn check_interval(&self) -> Option<Duration> {
        match self.check_interval.as_ref() {
            Some(interval) => {
                if let Some(minutes) = interval.minutes {
                    return Some(Duration::from_secs(minutes * 60));
                };

                None
            }
            None => None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct CheckInterval {
    minutes: Option<u64>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Documentation {
    url: String,
    #[serde(rename = "$value")]
    properties: Vec<DocumentationDescription>,
}

impl Documentation {
    /// Where the documentation can be found.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The documentation in different languages.
    pub fn properties(&self) -> &Vec<DocumentationDescription> {
        &self.properties
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct DocumentationDescription {
    lang: Option<String>,
    #[serde(rename = "$value")]
    description: String,
}

impl DocumentationDescription {
    /// What language the documentation is in.
    pub fn language(&self) -> Option<&str> {
        match &self.lang {
            Some(lang) => Some(lang),
            None => None,
        }
    }

    /// The documentation.
    pub fn description(&self) -> &str {
        &self.description
    }
}
