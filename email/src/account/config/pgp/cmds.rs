use mml::pgp::{Pgp, PgpCommands};
use process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct PgpCommandsConfig {
    pub encrypt_cmd: Option<Command>,
    pub encrypt_recipient_fmt: Option<String>,
    pub encrypt_recipients_sep: Option<String>,
    pub decrypt_cmd: Option<Command>,
    pub sign_cmd: Option<Command>,
    pub verify_cmd: Option<Command>,
}

impl From<PgpCommandsConfig> for Pgp {
    fn from(config: PgpCommandsConfig) -> Self {
        Pgp::Commands(PgpCommands {
            encrypt_cmd: config.encrypt_cmd,
            encrypt_recipient_fmt: config.encrypt_recipient_fmt,
            encrypt_recipients_sep: config.encrypt_recipients_sep,
            decrypt_cmd: config.decrypt_cmd,
            sign_cmd: config.sign_cmd,
            verify_cmd: config.verify_cmd,
        })
    }
}
