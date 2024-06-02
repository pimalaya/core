use std::{collections::HashMap, path::PathBuf};

use crate::{
    account::{config::AccountConfig, Error},
    Result,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case", deny_unknown_fields)
)]
pub struct Config {
    /// The default display name of the user.
    ///
    /// It usually corresponds to the full name of the user. This
    /// display name is used by default for all accounts.
    pub display_name: Option<String>,

    /// The default email signature of the user.
    ///
    /// It can be either a path to a file (usually `~/.signature`) or
    /// a raw string. This signature is used by default for all
    /// accounts.
    pub signature: Option<String>,

    /// The default email signature delimiter of the user signature.
    ///
    /// Defaults to `-- \n`. This signature delimiter is used by
    /// default for all accounts.
    pub signature_delim: Option<String>,

    /// The default downloads directory.
    ///
    /// It is mostly used for downloading messages
    /// attachments. Defaults to the system temporary directory
    /// (usually `/tmp`). This downloads directory is used by default
    /// for all accounts.
    pub downloads_dir: Option<PathBuf>,

    /// The map of account-specific configurations.
    pub accounts: HashMap<String, AccountConfig>,
}

impl Config {
    pub fn account(&self, name: impl AsRef<str>) -> Result<AccountConfig> {
        let name = name.as_ref();

        let account_config = self
            .accounts
            .get(name)
            .ok_or_else(|| Error::GetAccountConfigNotFoundError(name.to_owned()))?;

        Ok(AccountConfig {
            name: name.to_owned(),
            email: account_config.email.clone(),
            display_name: account_config
                .display_name
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| self.display_name.as_ref().map(ToOwned::to_owned)),
            signature_delim: account_config
                .signature_delim
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| self.signature_delim.as_ref().map(ToOwned::to_owned)),
            signature: account_config
                .signature
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| self.signature.as_ref().map(ToOwned::to_owned)),
            downloads_dir: account_config
                .downloads_dir
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| self.downloads_dir.as_ref().map(ToOwned::to_owned)),
            folder: account_config.folder.clone(),
            envelope: account_config.envelope.clone(),
            flag: account_config.flag.clone(),
            message: account_config.message.clone(),
            template: account_config.template.clone(),
            #[cfg(feature = "sync")]
            sync: account_config.sync.clone(),
            #[cfg(feature = "pgp")]
            pgp: account_config.pgp.clone(),
        })
    }
}
