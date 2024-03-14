use mml::pgp::{CmdsPgp, Pgp};
use process::Cmd;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct CmdsPgpConfig {
    pub encrypt_cmd: Option<Cmd>,
    pub encrypt_recipient_fmt: Option<String>,
    pub encrypt_recipients_sep: Option<String>,
    pub decrypt_cmd: Option<Cmd>,
    pub sign_cmd: Option<Cmd>,
    pub verify_cmd: Option<Cmd>,
}

impl From<CmdsPgpConfig> for Pgp {
    fn from(val: CmdsPgpConfig) -> Self {
        Pgp::Cmds(CmdsPgp {
            encrypt_cmd: val.encrypt_cmd,
            encrypt_recipient_fmt: val.encrypt_recipient_fmt,
            encrypt_recipients_sep: val.encrypt_recipients_sep,
            decrypt_cmd: val.decrypt_cmd,
            sign_cmd: val.sign_cmd,
            verify_cmd: val.verify_cmd,
        })
    }
}
