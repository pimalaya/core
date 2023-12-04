use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;

use crate::{
    account::config::AccountConfig,
    email::config::{EmailHooks, EmailTextPlainFormat},
    Result,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get configuration of account {0}")]
    GetAccountConfigNotFoundError(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Config {
    pub display_name: Option<String>,
    pub signature_delim: Option<String>,
    pub signature: Option<String>,
    pub downloads_dir: Option<PathBuf>,

    pub folder_listing_page_size: Option<usize>,
    pub folder_aliases: Option<HashMap<String, String>>,

    pub email_listing_page_size: Option<usize>,
    pub email_listing_datetime_fmt: Option<String>,
    pub email_listing_datetime_local_tz: Option<bool>,
    pub email_reading_headers: Option<Vec<String>>,
    pub email_reading_format: Option<EmailTextPlainFormat>,
    pub email_writing_headers: Option<Vec<String>>,
    pub email_sending_save_copy: Option<bool>,
    pub email_hooks: Option<EmailHooks>,

    pub accounts: HashMap<String, AccountConfig>,
}

impl Config {
    pub fn account(&self, name: impl AsRef<str>) -> Result<AccountConfig> {
        let name = name.as_ref();

        let account_config = self
            .accounts
            .get(name)
            .ok_or_else(|| Error::GetAccountConfigNotFoundError(name.to_owned()))?;

        let mut folder_aliases = account_config.folder_aliases.clone();

        folder_aliases.extend(
            self.folder_aliases
                .as_ref()
                .map(ToOwned::to_owned)
                .unwrap_or_default(),
        );

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
            folder_listing_page_size: account_config
                .folder_listing_page_size
                .or_else(|| self.folder_listing_page_size),
            folder_aliases,
            email_listing_page_size: account_config
                .email_listing_page_size
                .or_else(|| self.email_listing_page_size),
            email_listing_datetime_fmt: account_config
                .email_listing_datetime_fmt
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| {
                    self.email_listing_datetime_fmt
                        .as_ref()
                        .map(ToOwned::to_owned)
                }),
            email_listing_datetime_local_tz: account_config
                .email_listing_datetime_local_tz
                .or_else(|| self.email_listing_datetime_local_tz),
            email_reading_headers: account_config
                .email_reading_headers
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| self.email_reading_headers.as_ref().map(ToOwned::to_owned)),
            email_reading_format: account_config.email_reading_format.clone(),
            email_writing_headers: account_config
                .email_writing_headers
                .as_ref()
                .map(ToOwned::to_owned)
                .or_else(|| self.email_writing_headers.as_ref().map(ToOwned::to_owned)),
            email_sending_save_copy: account_config
                .email_sending_save_copy
                .or(self.email_sending_save_copy),
            email_hooks: EmailHooks {
                pre_send: account_config.email_hooks.pre_send.clone(),
            },
            sync: account_config.sync,
            sync_dir: account_config.sync_dir.clone(),
            sync_folders_strategy: account_config.sync_folders_strategy.clone(),

            #[cfg(feature = "pgp")]
            pgp: account_config.pgp.clone(),
        })
    }
}
