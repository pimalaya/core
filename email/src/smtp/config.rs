//! Module dedicated to the SMTP sender configuration.
//!
//! This module contains the configuration specific to the SMTP
//! sender.

use log::debug;
use mail_send::Credentials;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::{fmt, io, marker::PhantomData, result};
use thiserror::Error;

use crate::{
    account::config::{
        oauth2::{OAuth2Config, OAuth2Method},
        passwd::PasswdConfig,
    },
    Result,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get smtp password")]
    GetPasswdError(#[source] secret::Error),
    #[error("cannot get smtp password: password is empty")]
    GetPasswdEmptyError,
}

/// The SMTP sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SmtpConfig {
    /// The SMTP server host name.
    pub host: String,

    /// The SMTP server host port.
    pub port: u16,

    /// The SMTP encryption protocol to use.
    ///
    /// Supported encryption: SSL/TLS or STARTTLS.
    #[serde(default, deserialize_with = "some_bool_or_kind")]
    pub encryption: Option<SmtpEncryptionKind>,

    /// The SMTP server login.
    ///
    /// Usually, the login is either the email address or its left
    /// part (before @).
    pub login: String,

    /// The SMTP server authentication configuration.
    ///
    /// Authentication can be done using password or OAuth 2.0.
    /// See [SmtpAuthConfig].
    #[serde(flatten)]
    pub auth: SmtpAuthConfig,
}

impl SmtpConfig {
    /// Return `true` if TLS or StartTLS is enabled.
    pub fn is_encryption_enabled(&self) -> bool {
        match self.encryption.as_ref() {
            None => true,
            Some(SmtpEncryptionKind::Tls) => true,
            Some(SmtpEncryptionKind::StartTls) => true,
            _ => false,
        }
    }

    /// Return `true` if StartTLS is enabled.
    pub fn is_start_tls_encryption_enabled(&self) -> bool {
        match self.encryption.as_ref() {
            Some(SmtpEncryptionKind::StartTls) => true,
            _ => false,
        }
    }

    /// Return `true` if encryption is disabled.
    pub fn is_encryption_disabled(&self) -> bool {
        match self.encryption.as_ref() {
            Some(SmtpEncryptionKind::None) => true,
            _ => false,
        }
    }

    /// Builds the SMTP credentials string.
    ///
    /// The result depends on the [`SmtpAuthConfig`]: if password mode
    /// then creates credentials from login/password, if OAuth 2.0
    /// then creates credentials from access token.
    pub async fn credentials(&self) -> Result<Credentials<String>> {
        Ok(match &self.auth {
            SmtpAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().await.map_err(Error::GetPasswdError)?;
                let passwd = passwd.lines().next().ok_or(Error::GetPasswdEmptyError)?;
                Credentials::new(self.login.clone(), passwd.to_owned())
            }
            SmtpAuthConfig::OAuth2(oauth2) => match oauth2.method {
                OAuth2Method::XOAuth2 => {
                    Credentials::new_xoauth2(self.login.clone(), oauth2.access_token().await?)
                }
                OAuth2Method::OAuthBearer => Credentials::new_oauth(oauth2.access_token().await?),
            },
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SmtpEncryptionKind {
    #[default]
    #[serde(alias = "ssl")]
    Tls,
    #[serde(alias = "starttls")]
    StartTls,
    None,
}

impl fmt::Display for SmtpEncryptionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tls => write!(f, "SSL/TLS"),
            Self::StartTls => write!(f, "StartTLS"),
            Self::None => write!(f, "None"),
        }
    }
}

impl From<bool> for SmtpEncryptionKind {
    fn from(value: bool) -> Self {
        if value {
            Self::Tls
        } else {
            Self::None
        }
    }
}

/// The SMTP authentication configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtpAuthConfig {
    /// The password authentication mechanism.
    #[serde(alias = "password")]
    Passwd(PasswdConfig),

    /// The OAuth 2.0 authentication mechanism.
    OAuth2(OAuth2Config),
}

impl Default for SmtpAuthConfig {
    fn default() -> Self {
        Self::Passwd(PasswdConfig::default())
    }
}

impl SmtpAuthConfig {
    /// Resets the OAuth 2.0 authentication tokens.
    pub async fn reset(&self) -> Result<()> {
        debug!("resetting smtp backend configuration");

        if let Self::OAuth2(oauth2) = self {
            oauth2.reset().await?;
        }

        Ok(())
    }

    /// Configures the OAuth 2.0 authentication tokens.
    pub async fn configure(
        &self,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        debug!("configuring smtp backend");

        if let Self::OAuth2(oauth2) = self {
            oauth2.configure(get_client_secret).await?;
        }

        Ok(())
    }

    pub fn replace_undefined_keyring_entries(&mut self, name: impl AsRef<str>) {
        let name = name.as_ref();

        match self {
            SmtpAuthConfig::Passwd(secret) => {
                secret.set_keyring_entry_if_undefined(format!("{name}-smtp-passwd"));
            }
            SmtpAuthConfig::OAuth2(config) => {
                config
                    .client_secret
                    .set_keyring_entry_if_undefined(format!("{name}-smtp-oauth2-client-secret"));
                config
                    .access_token
                    .set_keyring_entry_if_undefined(format!("{name}-smtp-oauth2-access-token"));
                config
                    .refresh_token
                    .set_keyring_entry_if_undefined(format!("{name}-smtp-oauth2-refresh-token"));
            }
        }
    }
}

fn some_bool_or_kind<'de, D>(
    deserializer: D,
) -> result::Result<Option<SmtpEncryptionKind>, D::Error>
where
    D: Deserializer<'de>,
{
    struct SomeBoolOrKind(PhantomData<fn() -> Option<SmtpEncryptionKind>>);

    impl<'de> de::Visitor<'de> for SomeBoolOrKind {
        type Value = Option<SmtpEncryptionKind>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("some or none")
        }

        fn visit_some<D>(self, deserializer: D) -> result::Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct BoolOrKind(PhantomData<fn() -> SmtpEncryptionKind>);

            impl<'de> de::Visitor<'de> for BoolOrKind {
                type Value = SmtpEncryptionKind;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("boolean or string")
                }

                fn visit_bool<E>(self, v: bool) -> result::Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(v.into())
                }

                fn visit_str<E>(self, v: &str) -> result::Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Deserialize::deserialize(de::value::StrDeserializer::new(v))
                }
            }

            deserializer
                .deserialize_any(BoolOrKind(PhantomData))
                .map(Option::Some)
        }
    }

    deserializer.deserialize_option(SomeBoolOrKind(PhantomData))
}
