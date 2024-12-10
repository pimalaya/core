//! Module dedicated to the SMTP sender configuration.
//!
//! This module contains the configuration specific to the SMTP
//! sender.

use std::io;

use mail_send::Credentials;
use tracing::debug;

#[doc(inline)]
pub use super::{Error, Result};
#[cfg(feature = "oauth2")]
use crate::account::config::oauth2::{OAuth2Config, OAuth2Method};
use crate::{account::config::passwd::PasswordConfig, tls::Encryption};

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
    pub encryption: Option<Encryption>,

    /// The SMTP server login.
    ///
    /// Usually, the login is either the email address or its left
    /// part (before @).
    pub login: String,

    /// The SMTP server authentication configuration.
    ///
    /// Authentication can be done using password or OAuth 2.0.
    /// See [SmtpAuthConfig].
    pub auth: SmtpAuthConfig,
}

impl SmtpConfig {
    /// Return `true` if TLS or StartTLS is enabled.
    pub fn is_encryption_enabled(&self) -> bool {
        matches!(
            self.encryption.as_ref(),
            None | Some(Encryption::Tls(_)) | Some(Encryption::StartTls(_))
        )
    }

    /// Return `true` if StartTLS is enabled.
    pub fn is_start_tls_encryption_enabled(&self) -> bool {
        matches!(self.encryption.as_ref(), Some(Encryption::StartTls(_)))
    }

    /// Return `true` if encryption is disabled.
    pub fn is_encryption_disabled(&self) -> bool {
        matches!(self.encryption.as_ref(), Some(Encryption::None))
    }

    /// Builds the SMTP credentials string.
    ///
    /// The result depends on the [`SmtpAuthConfig`]: if password mode
    /// then creates credentials from login/password, if OAuth 2.0
    /// then creates credentials from access token.
    pub async fn credentials(&self) -> Result<Credentials<String>> {
        Ok(match &self.auth {
            SmtpAuthConfig::Password(passwd) => {
                let passwd = passwd.get().await.map_err(Error::GetPasswdSmtpError)?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or(Error::GetPasswdEmptySmtpError)?;
                Credentials::new(self.login.clone(), passwd.to_owned())
            }
            #[cfg(feature = "oauth2")]
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

/// The SMTP authentication configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase"),
    serde(tag = "type"),
    serde(from = "SmtpAuthConfigDerive")
)]
pub enum SmtpAuthConfig {
    /// The password authentication mechanism.
    Password(PasswordConfig),

    /// The OAuth 2.0 authentication mechanism.
    #[cfg(feature = "oauth2")]
    OAuth2(OAuth2Config),
}

impl SmtpAuthConfig {
    /// Resets the OAuth 2.0 authentication tokens.
    pub async fn reset(&mut self) -> Result<()> {
        debug!("resetting smtp backend configuration");

        #[cfg(feature = "oauth2")]
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
        #[cfg_attr(not(feature = "oauth2"), allow(unused_variables))]
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        debug!("configuring smtp backend");

        #[cfg(feature = "oauth2")]
        if let Self::OAuth2(oauth2) = self {
            oauth2
                .configure(get_client_secret)
                .await
                .map_err(|_| Error::ConfiguringOAuthFailed)?;
        }

        Ok(())
    }

    #[cfg(feature = "keyring")]
    pub fn replace_empty_secrets(&mut self, name: impl AsRef<str>) -> Result<()> {
        let name = name.as_ref();

        match self {
            SmtpAuthConfig::Password(secret) => {
                secret
                    .replace_with_keyring_if_empty(format!("{name}-smtp-passwd"))
                    .map_err(Error::ReplacingKeyringFailed)?;
            }
            #[cfg(feature = "oauth2")]
            SmtpAuthConfig::OAuth2(config) => {
                if let Some(secret) = config.client_secret.as_mut() {
                    secret
                        .replace_with_keyring_if_empty(format!("{name}-smtp-oauth2-client-secret"))
                        .map_err(Error::ReplacingKeyringFailed)?;
                }

                config
                    .access_token
                    .replace_with_keyring_if_empty(format!("{name}-smtp-oauth2-access-token"))
                    .map_err(Error::ReplacingKeyringFailed)?;
                config
                    .refresh_token
                    .replace_with_keyring_if_empty(format!("{name}-smtp-oauth2-refresh-token"))
                    .map_err(Error::ReplacingKeyringFailed)?;
            }
        }

        Ok(())
    }
}

impl Default for SmtpAuthConfig {
    fn default() -> Self {
        Self::Password(PasswordConfig::default())
    }
}

#[cfg(feature = "derive")]
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum SmtpAuthConfigDerive {
    Password(PasswordConfig),
    #[cfg(feature = "oauth2")]
    OAuth2(OAuth2Config),
    #[cfg(not(feature = "oauth2"))]
    #[serde(skip_serializing, deserialize_with = "missing_oauth2_feature")]
    OAuth2,
}

#[cfg(all(feature = "derive", not(feature = "oauth2")))]
fn missing_oauth2_feature<'de, D>(_: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    Err(serde::de::Error::custom("missing `oauth2` cargo feature"))
}

#[cfg(feature = "derive")]
impl From<SmtpAuthConfigDerive> for SmtpAuthConfig {
    fn from(config: SmtpAuthConfigDerive) -> Self {
        match config {
            SmtpAuthConfigDerive::Password(config) => Self::Password(config),
            #[cfg(feature = "oauth2")]
            SmtpAuthConfigDerive::OAuth2(config) => Self::OAuth2(config),
            #[cfg(not(feature = "oauth2"))]
            SmtpAuthConfigDerive::OAuth2 => unreachable!(),
        }
    }
}
