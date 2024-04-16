//! Module dedicated to the SMTP sender configuration.
//!
//! This module contains the configuration specific to the SMTP
//! sender.

use crate::debug;
use mail_send::Credentials;
use std::{fmt, io};
#[cfg(feature = "derive")]
use std::{marker::PhantomData, result};

use crate::account::config::{
    oauth2::{OAuth2Config, OAuth2Method},
    passwd::PasswdConfig,
};

#[doc(inline)]
pub use super::{Error, Result};

/// The SMTP sender configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct SmtpConfig {
    /// The SMTP server host name.
    pub host: String,

    /// The SMTP server host port.
    pub port: u16,

    /// The SMTP encryption protocol to use.
    ///
    /// Supported encryption: SSL/TLS or STARTTLS.
    #[cfg_attr(
        feature = "derive",
        serde(default, deserialize_with = "some_bool_or_kind")
    )]
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
    #[cfg_attr(feature = "derive", serde(flatten))]
    pub auth: SmtpAuthConfig,
}

impl SmtpConfig {
    /// Return `true` if TLS or StartTLS is enabled.
    pub fn is_encryption_enabled(&self) -> bool {
        matches!(
            self.encryption.as_ref(),
            None | Some(SmtpEncryptionKind::Tls) | Some(SmtpEncryptionKind::StartTls)
        )
    }

    /// Return `true` if StartTLS is enabled.
    pub fn is_start_tls_encryption_enabled(&self) -> bool {
        matches!(self.encryption.as_ref(), Some(SmtpEncryptionKind::StartTls))
    }

    /// Return `true` if encryption is disabled.
    pub fn is_encryption_disabled(&self) -> bool {
        matches!(self.encryption.as_ref(), Some(SmtpEncryptionKind::None))
    }

    /// Builds the SMTP credentials string.
    ///
    /// The result depends on the [`SmtpAuthConfig`]: if password mode
    /// then creates credentials from login/password, if OAuth 2.0
    /// then creates credentials from access token.
    pub async fn credentials(&self) -> Result<Credentials<String>> {
        Ok(match &self.auth {
            SmtpAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().await.map_err(Error::GetPasswdSmtpError)?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or(Error::GetPasswdEmptySmtpError)?;
                Credentials::new(self.login.clone(), passwd.to_owned())
            }
            SmtpAuthConfig::OAuth2(oauth2) => {
                let access_token = oauth2
                    .access_token()
                    .await
                    .map_err(|_| Error::AccessTokenWasNotAvailable)?;

                match oauth2.method {
                    OAuth2Method::XOAuth2 => {
                        Credentials::new_xoauth2(self.login.clone(), access_token)
                    }
                    OAuth2Method::OAuthBearer => Credentials::new_oauth(access_token),
                }
            }
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum SmtpEncryptionKind {
    #[default]
    #[cfg_attr(feature = "derive", serde(alias = "ssl"))]
    Tls,
    #[cfg_attr(feature = "derive", serde(alias = "starttls"))]
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
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]

pub enum SmtpAuthConfig {
    /// The password authentication mechanism.
    #[cfg_attr(feature = "derive", serde(alias = "password"))]
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
    pub async fn reset(&mut self) -> Result<()> {
        debug!("resetting smtp backend configuration");

        if let Self::OAuth2(oauth2) = self {
            oauth2
                .reset()
                .await
                .map_err(|_| Error::ResettingOAuthFailed)?;
        }

        Ok(())
    }

    /// Configures the OAuth 2.0 authentication tokens.
    pub async fn configure(
        &mut self,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        debug!("configuring smtp backend");

        if let Self::OAuth2(oauth2) = self {
            oauth2
                .configure(get_client_secret)
                .await
                .map_err(|_| Error::ConfiguringOAuthFailed)?;
        }

        Ok(())
    }

    pub fn replace_undefined_keyring_entries(&mut self, name: impl AsRef<str>) -> Result<()> {
        let name = name.as_ref();

        match self {
            SmtpAuthConfig::Passwd(secret) => {
                secret
                    .replace_undefined_to_keyring(format!("{name}-smtp-passwd"))
                    .map_err(Error::ReplacingKeyringFailed)?;
            }
            SmtpAuthConfig::OAuth2(config) => {
                config
                    .client_secret
                    .replace_undefined_to_keyring(format!("{name}-smtp-oauth2-client-secret"))
                    .map_err(Error::ReplacingKeyringFailed)?;
                config
                    .access_token
                    .replace_undefined_to_keyring(format!("{name}-smtp-oauth2-access-token"))
                    .map_err(Error::ReplacingKeyringFailed)?;
                config
                    .refresh_token
                    .replace_undefined_to_keyring(format!("{name}-smtp-oauth2-refresh-token"))
                    .map_err(Error::ReplacingKeyringFailed)?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "derive")]
fn some_bool_or_kind<'de, D>(
    deserializer: D,
) -> result::Result<Option<SmtpEncryptionKind>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SomeBoolOrKind(PhantomData<fn() -> Option<SmtpEncryptionKind>>);

    impl<'de> serde::de::Visitor<'de> for SomeBoolOrKind {
        type Value = Option<SmtpEncryptionKind>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("some or none")
        }

        fn visit_some<D>(self, deserializer: D) -> result::Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct BoolOrKind(PhantomData<fn() -> SmtpEncryptionKind>);

            impl<'de> serde::de::Visitor<'de> for BoolOrKind {
                type Value = SmtpEncryptionKind;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("boolean or string")
                }

                fn visit_bool<E>(self, v: bool) -> result::Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    Ok(v.into())
                }

                fn visit_str<E>(self, v: &str) -> result::Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    serde::Deserialize::deserialize(serde::de::value::StrDeserializer::new(v))
                }
            }

            deserializer
                .deserialize_any(BoolOrKind(PhantomData))
                .map(Option::Some)
        }
    }

    deserializer.deserialize_option(SomeBoolOrKind(PhantomData))
}
