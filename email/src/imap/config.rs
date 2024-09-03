//! Module dedicated to the IMAP backend configuration.
//!
//! This module contains the implementation of the IMAP backend and
//! all associated structures related to it.

use std::fmt;
#[cfg(feature = "derive")]
use std::{marker::PhantomData, result};

#[doc(inline)]
use super::{Error, Result};
#[cfg(feature = "oauth2")]
use crate::account::config::oauth2::OAuth2Config;
use crate::account::config::passwd::PasswdConfig;

/// Errors related to the IMAP backend configuration.

/// The IMAP backend configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ImapConfig {
    /// The IMAP server host name.
    pub host: String,

    /// The IMAP server host port.
    pub port: u16,

    /// The IMAP encryption protocol to use.
    ///
    /// Supported encryption: SSL/TLS, STARTTLS or none.
    #[cfg_attr(
        feature = "derive",
        serde(default, deserialize_with = "some_bool_or_kind")
    )]
    pub encryption: Option<ImapEncryptionKind>,

    /// The IMAP server login.
    ///
    /// Usually, the login is either the email address or its left
    /// part (before @).
    pub login: String,

    /// The IMAP server authentication configuration.
    ///
    /// Authentication can be done using password or OAuth 2.0.
    /// See [ImapAuthConfig].
    pub auth: ImapAuthConfig,

    /// The IMAP extensions configuration.
    pub extensions: Option<ImapExtensionsConfig>,

    /// The IMAP notify command.
    ///
    /// Defines the command used to notify the user when a new email is available.
    /// Defaults to `notify-send "📫 <sender>" "<subject>"`.
    pub watch: Option<ImapWatchConfig>,
}

impl ImapConfig {
    pub fn send_id_after_auth(&self) -> bool {
        self.extensions
            .as_ref()
            .and_then(|ext| ext.id.as_ref())
            .and_then(|id| id.send_after_auth)
            .unwrap_or_default()
    }

    /// Return `true` if TLS or StartTLS is enabled.
    pub fn is_encryption_enabled(&self) -> bool {
        matches!(
            self.encryption.as_ref(),
            None | Some(ImapEncryptionKind::Tls) | Some(ImapEncryptionKind::StartTls)
        )
    }

    /// Return `true` if StartTLS is enabled.
    pub fn is_start_tls_encryption_enabled(&self) -> bool {
        matches!(self.encryption.as_ref(), Some(ImapEncryptionKind::StartTls))
    }

    /// Return `true` if encryption is disabled.
    pub fn is_encryption_disabled(&self) -> bool {
        matches!(self.encryption.as_ref(), Some(ImapEncryptionKind::None))
    }

    /// Builds authentication credentials.
    ///
    /// Authentication credentials can be either a password or an
    /// OAuth 2.0 access token.
    pub async fn build_credentials(&self) -> Result<String> {
        self.auth.build_credentials().await
    }

    /// Find the IMAP watch timeout.
    pub fn find_watch_timeout(&self) -> Option<u64> {
        self.watch.as_ref().and_then(|c| c.find_timeout())
    }
}

#[cfg(feature = "sync")]
impl crate::sync::hash::SyncHash for ImapConfig {
    fn sync_hash(&self, state: &mut std::hash::DefaultHasher) {
        std::hash::Hash::hash(&self.host, state);
        std::hash::Hash::hash(&self.port, state);
        std::hash::Hash::hash(&self.login, state);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum ImapEncryptionKind {
    #[default]
    #[cfg_attr(feature = "derive", serde(alias = "ssl"))]
    Tls,
    #[cfg_attr(feature = "derive", serde(alias = "starttls"))]
    StartTls,
    None,
}

impl fmt::Display for ImapEncryptionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tls => write!(f, "SSL/TLS"),
            Self::StartTls => write!(f, "StartTLS"),
            Self::None => write!(f, "None"),
        }
    }
}

impl From<bool> for ImapEncryptionKind {
    fn from(value: bool) -> Self {
        if value {
            Self::Tls
        } else {
            Self::None
        }
    }
}

/// The IMAP authentication configuration.
///
/// Authentication can be done using password or OAuth 2.0.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase", tag = "type")
)]
pub enum ImapAuthConfig {
    /// The password configuration.
    #[cfg_attr(feature = "derive", serde(alias = "password"))]
    Passwd(PasswdConfig),

    /// The OAuth 2.0 configuration.
    #[cfg(feature = "oauth2")]
    OAuth2(OAuth2Config),
}

impl Default for ImapAuthConfig {
    fn default() -> Self {
        Self::Passwd(Default::default())
    }
}

impl ImapAuthConfig {
    /// Reset IMAP secrets (password or OAuth 2.0 tokens).
    pub async fn reset(&self) -> Result<()> {
        match self {
            ImapAuthConfig::Passwd(config) => {
                config.reset().await.map_err(Error::ResetPasswordError)
            }
            #[cfg(feature = "oauth2")]
            ImapAuthConfig::OAuth2(config) => {
                config.reset().await.map_err(Error::ResetOAuthSecretsError)
            }
        }
    }

    /// Builds authentication credentials.
    ///
    /// Authentication credentials can be either a password or an
    /// OAuth 2.0 access token.
    pub async fn build_credentials(&self) -> Result<String> {
        match self {
            ImapAuthConfig::Passwd(passwd) => {
                let passwd = passwd.get().await.map_err(Error::GetPasswdImapError)?;
                let passwd = passwd
                    .lines()
                    .next()
                    .ok_or(Error::GetPasswdEmptyImapError)?;
                Ok(passwd.to_owned())
            }
            #[cfg(feature = "oauth2")]
            ImapAuthConfig::OAuth2(oauth2) => Ok(oauth2
                .access_token()
                .await
                .map_err(Error::AccessTokenNotAvailable)?),
        }
    }

    #[cfg(feature = "keyring")]
    pub fn replace_undefined_keyring_entries(&mut self, name: impl AsRef<str>) -> Result<()> {
        let name = name.as_ref();

        match self {
            Self::Passwd(secret) => {
                secret
                    .replace_undefined_to_keyring(format!("{name}-imap-passwd"))
                    .map_err(Error::ReplacingUnidentifiedFailed)?;
            }
            #[cfg(feature = "oauth2")]
            Self::OAuth2(config) => {
                config
                    .client_secret
                    .replace_undefined_to_keyring(format!("{name}-imap-oauth2-client-secret"))
                    .map_err(Error::ReplacingUnidentifiedFailed)?;
                config
                    .access_token
                    .replace_undefined_to_keyring(format!("{name}-imap-oauth2-access-token"))
                    .map_err(Error::ReplacingUnidentifiedFailed)?;
                config
                    .refresh_token
                    .replace_undefined_to_keyring(format!("{name}-imap-oauth2-refresh-token"))
                    .map_err(Error::ReplacingUnidentifiedFailed)?;
            }
        }

        Ok(())
    }
}

/// The IMAP watch options (IDLE).
///
/// Options dedicated to the IMAP IDLE mode, which is used to watch
/// changes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ImapWatchConfig {
    /// The IMAP watch timeout.
    ///
    /// Timeout used to refresh the IDLE command in
    /// background. Defaults to 29 min as defined in the RFC.
    timeout: Option<u64>,
}

impl ImapWatchConfig {
    /// Find the IMAP watch timeout.
    pub fn find_timeout(&self) -> Option<u64> {
        self.timeout
    }
}

#[cfg(feature = "derive")]
fn some_bool_or_kind<'de, D>(
    deserializer: D,
) -> result::Result<Option<ImapEncryptionKind>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SomeBoolOrKind(PhantomData<fn() -> Option<ImapEncryptionKind>>);

    impl<'de> serde::de::Visitor<'de> for SomeBoolOrKind {
        type Value = Option<ImapEncryptionKind>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("some or none")
        }

        fn visit_some<D>(self, deserializer: D) -> result::Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct BoolOrKind(PhantomData<fn() -> ImapEncryptionKind>);

            impl<'de> serde::de::Visitor<'de> for BoolOrKind {
                type Value = ImapEncryptionKind;

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

/// The IMAP configuration dedicated to extensions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ImapExtensionsConfig {
    id: Option<ImapIdExtensionConfig>,
}

/// The IMAP configuration dedicated to the ID extension.
///
/// https://www.rfc-editor.org/rfc/rfc2971.html
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct ImapIdExtensionConfig {
    /// Automatically sends the ID command straight after
    /// authentication.
    send_after_auth: Option<bool>,
}
