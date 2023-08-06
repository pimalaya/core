use pimalaya_process::Cmd;
use thiserror::Error;

use crate::Result;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot encrypt data using commands")]
    EncryptError(#[source] pimalaya_process::Error),
    #[error("cannot decrypt data using commands")]
    DecryptError(#[source] pimalaya_process::Error),
    #[error("cannot sign data using commands")]
    SignError(#[source] pimalaya_process::Error),
    #[error("cannot verify data using commands")]
    VerifyError(#[source] pimalaya_process::Error),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CmdsPgp {
    pub encrypt_cmd: Option<Cmd>,
    pub encrypt_recipient_fmt: Option<String>,
    pub encrypt_recipients_sep: Option<String>,
    pub decrypt_cmd: Option<Cmd>,
    pub sign_cmd: Option<Cmd>,
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
