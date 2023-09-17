//! # PGP shell commands module
//!
//! This module contains the PGP backend based on shell commands.

use process::Cmd;
use thiserror::Error;

use crate::Result;

/// Errors dedicated to PGP shell commands.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot encrypt data using commands")]
    EncryptError(#[source] process::Error),
    #[error("cannot decrypt data using commands")]
    DecryptError(#[source] process::Error),
    #[error("cannot sign data using commands")]
    SignError(#[source] process::Error),
    #[error("cannot verify data using commands")]
    VerifyError(#[source] process::Error),
}

/// The shell commands PGP backend.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CmdsPgp {
    /// The PGP encrypt command.
    ///
    /// A special placeholder `<recipients>` is available to represent
    /// the recipients the message needs to be encrypted for. See [CmdsPgp::default_encrypt_cmd].
    ///
    /// Defaults to `gpg --encrypt --quiet --armor <recipients>`.
    pub encrypt_cmd: Option<Cmd>,

    /// The PGP encrypt recipient format.
    ///
    /// A special placeholder `<recipient>` is available to represent
    /// one recipient of the encrypt command.
    ///
    /// Default to `--recipient <recipient>`.
    pub encrypt_recipient_fmt: Option<String>,

    /// The PGP encrypt recipients separator.
    ///
    /// Separator used between recipient formats.
    ///
    /// Defaults to space.
    pub encrypt_recipients_sep: Option<String>,

    /// The PGP decrypt command.
    ///
    /// Defaults to `gpg --decrypt --quiet`.
    pub decrypt_cmd: Option<Cmd>,

    /// The PGP sign command.
    ///
    /// Default to `gpg --sign --quiet --armor`.
    pub sign_cmd: Option<Cmd>,

    /// The PGP verify command.
    ///
    /// Default to `gpg --verify --quiet`.
    pub verify_cmd: Option<Cmd>,
}

impl CmdsPgp {
    pub fn default_encrypt_cmd() -> Cmd {
        Cmd::from("gpg --encrypt --quiet --armor <recipients>")
    }

    pub fn default_encrypt_recipient_fmt() -> String {
        String::from("--recipient <recipient>")
    }

    pub fn default_encrypt_recipients_sep() -> String {
        String::from(" ")
    }

    pub fn default_decrypt_cmd() -> Cmd {
        Cmd::from("gpg --decrypt --quiet")
    }

    pub fn default_sign_cmd() -> Cmd {
        Cmd::from("gpg --sign --quiet --armor")
    }

    pub fn default_verify_cmd() -> Cmd {
        Cmd::from("gpg --verify --quiet")
    }

    /// Encrypts the given plain bytes using the given recipients.
    pub async fn encrypt(
        &self,
        recipients: impl IntoIterator<Item = String>,
        plain_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let recipient_fmt = self
            .encrypt_recipient_fmt
            .clone()
            .unwrap_or_else(Self::default_encrypt_recipient_fmt);
        let recipients_sep = self
            .encrypt_recipients_sep
            .clone()
            .unwrap_or_else(Self::default_encrypt_recipients_sep);
        let recipients_str =
            recipients
                .into_iter()
                .fold(String::new(), |mut recipients_str, recipient| {
                    if !recipients_str.is_empty() {
                        recipients_str.push_str(&recipients_sep);
                    }
                    recipients_str.push_str(&recipient_fmt.replace("<recipient>", &recipient));
                    recipients_str
                });

        let res = self
            .encrypt_cmd
            .clone()
            .unwrap_or_else(Self::default_encrypt_cmd)
            .replace("<recipients>", recipients_str)
            .run_with(plain_bytes)
            .await
            .map_err(Error::EncryptError)?;

        Ok(res.into())
    }

    /// Decrypts the given encrypted bytes.
    pub async fn decrypt(&self, encrypted_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let res = self
            .decrypt_cmd
            .clone()
            .unwrap_or_else(Self::default_decrypt_cmd)
            .run_with(encrypted_bytes)
            .await
            .map_err(Error::DecryptError)?;

        Ok(res.into())
    }

    /// Signs the given plain bytes.
    pub async fn sign(&self, plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let res = self
            .sign_cmd
            .clone()
            .unwrap_or_else(Self::default_sign_cmd)
            .run_with(plain_bytes)
            .await
            .map_err(Error::SignError)?;

        Ok(res.into())
    }

    /// Verifies the given signed bytes.
    pub async fn verify(&self, signature_bytes: Vec<u8>, _signed_bytes: Vec<u8>) -> Result<()> {
        self.verify_cmd
            .clone()
            .unwrap_or_else(Self::default_verify_cmd)
            .run_with(signature_bytes)
            .await
            .map_err(Error::VerifyError)?;

        Ok(())
    }
}
