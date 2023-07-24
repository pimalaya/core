use pimalaya_process::Cmd;
use thiserror::Error;

use crate::Result;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot sign data using pgp command")]
    SignError(#[from] pimalaya_process::Error),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpCmds {
    encrypt_cmd: Cmd,
    decrypt_cmd: Cmd,
    sign_cmd: Cmd,
    verify_cmd: Cmd,
}

impl PgpCmds {
    pub async fn sign(&self, data: &[u8], sender: impl ToString) -> Result<Vec<u8>> {
        let mut cmd = self.sign_cmd.clone();
        cmd = cmd.replace("<sender>", sender.to_string());
        Ok(cmd.run_with(data).await.map_err(Error::SignError)?.into())
    }
}
