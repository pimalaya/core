use process::Cmd;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FolderWatchConfig {
    /// Define the list of shell commands to execute when any change
    /// occurs on a folder (including emails).
    ///
    /// Commands are executed in serie, without any parameter nor
    /// input.
    pub change_hooks: Option<Vec<Cmd>>,
}
