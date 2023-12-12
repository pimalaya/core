use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{list::config::FolderListConfig, watch::config::FolderWatchConfig};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FolderConfig {
    /// Define custom folder aliases.
    ///
    /// Aliases are resolved when calling backend features. There are
    /// 4 special aliases that can be used by the lib, for example
    /// when saving a copy of a sent message to the `sent` folder:
    ///
    /// - `inbox`: main folder containing incoming messages
    /// - `draft(s)`: folder containing draft messages
    /// - `sent`: folder containing sent messages
    /// - `trash`: folder containing trashed messages
    pub aliases: Option<HashMap<String, String>>,

    /// The folder config related to listing.
    pub list: Option<FolderListConfig>,

    /// The folder config related to watching.
    pub watch: Option<FolderWatchConfig>,
}
