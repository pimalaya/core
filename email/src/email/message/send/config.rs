use process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct MessageSendConfig {
    /// Should save a copy to the sent folder of the message being
    /// sent.
    pub save_copy: Option<bool>,

    /// The hook called just before sending a message.
    ///
    /// The command should take a raw message as standard input
    /// (stdin) and returns the modified raw message to the standard
    /// output (stdout).
    pub pre_hook: Option<Command>,
}
