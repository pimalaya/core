use mml::pgp::{CmdsPgp, Pgp};
use process::Cmd;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CmdsPgpConfig {
    pub encrypt_cmd: Option<Cmd>,
    pub encrypt_recipient_fmt: Option<String>,
    pub encrypt_recipients_sep: Option<String>,
    pub decrypt_cmd: Option<Cmd>,
    pub sign_cmd: Option<Cmd>,
    pub verify_cmd: Option<Cmd>,
}

impl Into<Pgp> for CmdsPgpConfig {
    fn into(self) -> Pgp {
        Pgp::Cmds(CmdsPgp {
            encrypt_cmd: self.encrypt_cmd,
            encrypt_recipient_fmt: self.encrypt_recipient_fmt,
            encrypt_recipients_sep: self.encrypt_recipients_sep,
            decrypt_cmd: self.decrypt_cmd,
            sign_cmd: self.sign_cmd,
            verify_cmd: self.verify_cmd,
        })
    }
}
