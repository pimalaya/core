//! Module dedicated to account configuration.
//!
//! This module contains the representation of the user's current
//! account configuration named [`AccountConfig`].

#[cfg(feature = "oauth2")]
pub mod oauth2;
pub mod passwd;
#[cfg(feature = "pgp")]
pub mod pgp;

use std::{
    collections::HashMap,
    env::temp_dir,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    vec,
};

#[cfg(feature = "sync")]
use dirs::data_dir;
use dirs::download_dir;
use mail_builder::headers::address::{Address, EmailAddress};
use mail_parser::Address::*;
use mml::MimeInterpreterBuilder;
#[cfg(feature = "notify")]
use notify_rust::Notification;
use process::Command;
use shellexpand_utils::{shellexpand_path, shellexpand_str, try_shellexpand_path};
use tracing::debug;

#[cfg(feature = "pgp")]
use self::pgp::PgpConfig;
#[cfg(feature = "sync")]
use super::sync::config::SyncConfig;
#[doc(inline)]
pub use super::{Error, Result};
use crate::{
    date::from_mail_parser_to_chrono_datetime,
    email::config::EmailTextPlainFormat,
    envelope::{config::EnvelopeConfig, Envelope},
    flag::config::FlagConfig,
    folder::{config::FolderConfig, FolderKind, DRAFTS, INBOX, SENT, TRASH},
    message::config::MessageConfig,
    template::{
        config::TemplateConfig,
        forward::config::{ForwardTemplatePostingStyle, ForwardTemplateSignatureStyle},
        new::config::NewTemplateSignatureStyle,
        reply::config::{ReplyTemplatePostingStyle, ReplyTemplateSignatureStyle},
    },
    watch::config::WatchHook,
};

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIGNATURE_DELIM: &str = "-- \n";

pub trait HasAccountConfig {
    fn account_config(&self) -> &AccountConfig;
}

/// The user's account configuration.
///
/// It represents everything that the user can customize for a given
/// account. It is the main configuration used by all other
/// modules. Usually, it serves as a reference for building config
/// file structure.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case", deny_unknown_fields)
)]
pub struct AccountConfig {
    /// The name of the user account.
    ///
    /// The account name is used as an unique identifier for a given
    /// configuration.
    pub name: String,

    /// The email address of the user account.
    pub email: String,

    /// The display name of the user.
    ///
    /// It usually corresponds to the full name of the user.
    pub display_name: Option<String>,

    /// The email signature of the user.
    ///
    /// It can be either a path to a file (usually `~/.signature`) or
    /// a raw string.
    pub signature: Option<String>,

    /// The email signature delimiter of the user signature.
    ///
    /// Defaults to `-- \n`.
    pub signature_delim: Option<String>,

    /// The downloads directory.
    ///
    /// It is mostly used for downloading messages
    /// attachments. Defaults to the system temporary directory
    /// (usually `/tmp`).
    pub downloads_dir: Option<PathBuf>,

    /// The folder configuration.
    pub folder: Option<FolderConfig>,

    /// The envelope configuration.
    pub envelope: Option<EnvelopeConfig>,

    /// The flag configuration.
    pub flag: Option<FlagConfig>,

    /// The message configuration.
    pub message: Option<MessageConfig>,

    /// The message configuration.
    pub template: Option<TemplateConfig>,

    /// The account synchronization configuration.
    #[cfg(feature = "sync")]
    pub sync: Option<SyncConfig>,

    /// The PGP configuration.
    #[cfg(feature = "pgp")]
    pub pgp: Option<PgpConfig>,
}

impl AccountConfig {
    /// Get the signature, including the delimiter.
    ///
    /// Uses the default delimiter `-- \n` in case no delimiter has
    /// been defined. Return `None` if no signature has been defined.
    pub fn find_full_signature(&self) -> Option<String> {
        let delim = self
            .signature_delim
            .as_deref()
            .unwrap_or(DEFAULT_SIGNATURE_DELIM);

        let signature = self.signature.as_ref();

        signature.map(|path_or_raw| {
            let signature = try_shellexpand_path(path_or_raw)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
                .and_then(fs::read_to_string)
                .unwrap_or_else(|_err| {
                    debug!("cannot read signature from path: {_err}");
                    debug!("{_err:?}");
                    shellexpand_str(path_or_raw)
                });
            format!("{}{}", delim, signature.trim())
        })
    }

    /// Get then expand the downloads directory path.
    ///
    /// Falls back to [`dirs::download_dir`].
    pub fn get_downloads_dir(&self) -> PathBuf {
        self.downloads_dir
            .as_ref()
            .map(shellexpand_path)
            .or_else(download_dir)
            .unwrap_or_else(temp_dir)
    }

    /// Build the downloadable version of the given path.
    ///
    /// The aim of this helper is to build a safe download path for a
    /// given path.
    ///
    /// First, only the file name of the give path is taken in order
    /// to prevent any interaction outside of the downloads directory.
    ///
    /// Then, a suffix may be added to the final path if it already
    /// exists on the filesystem in order to prevent any overriding or
    /// data loss.
    pub fn get_download_file_path(&self, path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = path.as_ref();

        let file_name = path
            .file_name()
            .ok_or_else(|| Error::GetFileNameFromPathSyncError(path.to_owned()))?;

        let final_path = self.get_downloads_dir().join(file_name);

        rename_file_if_duplicate(&final_path, |path, _count| path.is_file())
    }

    /// Return `true` if the synchronization is enabled.
    #[cfg(feature = "sync")]
    pub fn is_sync_enabled(&self) -> bool {
        self.sync
            .as_ref()
            .and_then(|c| c.enable)
            .unwrap_or_default()
    }

    /// Return `true` if the synchronization directory already exists.
    #[cfg(feature = "sync")]
    pub fn does_sync_dir_exist(&self) -> bool {
        match self.sync.as_ref().and_then(|c| c.dir.as_ref()) {
            Some(dir) => try_shellexpand_path(dir).is_ok(),
            None => data_dir()
                .map(|dir| {
                    dir.join("pimalaya")
                        .join("email")
                        .join("sync")
                        .join(&self.name)
                        .is_dir()
                })
                .unwrap_or_default(),
        }
    }

    /// Execute the envelope received hook.
    #[cfg(feature = "watch")]
    pub async fn exec_received_envelope_hook(&self, envelope: &Envelope) {
        let hook = self
            .envelope
            .as_ref()
            .and_then(|c| c.watch.as_ref())
            .and_then(|c| c.received.as_ref());

        if let Some(hook) = hook.as_ref() {
            self.exec_envelope_hook(hook, envelope).await
        }
    }

    /// Execute the envelope any hook.
    #[cfg(feature = "watch")]
    pub async fn exec_any_envelope_hook(&self, envelope: &Envelope) {
        let hook = self
            .envelope
            .as_ref()
            .and_then(|c| c.watch.as_ref())
            .and_then(|c| c.any.as_ref());

        if let Some(hook) = hook.as_ref() {
            self.exec_envelope_hook(hook, envelope).await
        }
    }

    /// Execute the given envelope hook.
    pub async fn exec_envelope_hook(&self, hook: &WatchHook, envelope: &Envelope) {
        let sender = envelope.from.name.as_deref().unwrap_or(&envelope.from.addr);
        let sender_name = envelope.from.name.as_deref().unwrap_or("unknown");
        let recipient = envelope.to.name.as_deref().unwrap_or(&envelope.to.addr);
        let recipient_name = envelope.to.name.as_deref().unwrap_or("unknown");

        if let Some(cmd) = hook.cmd.as_ref() {
            let res = cmd
                .clone()
                .replace("{id}", &envelope.id)
                .replace("{subject}", &envelope.subject)
                .replace("{sender}", sender)
                .replace("{sender.name}", sender_name)
                .replace("{sender.address}", &envelope.from.addr)
                .replace("{recipient}", recipient)
                .replace("{recipient.name}", recipient_name)
                .replace("{recipient.address}", &envelope.to.addr)
                .run()
                .await;

            if let Err(_err) = res {
                debug!("error while executing watch command hook");
                debug!("{_err:?}");
            }
        }

        #[allow(unused_variables)]
        let replace = move |fmt: &str, envelope: &Envelope| -> String {
            fmt.replace("{id}", &envelope.id)
                .replace("{subject}", &envelope.subject)
                .replace("{sender}", sender)
                .replace("{sender.name}", sender_name)
                .replace("{sender.address}", &envelope.from.addr)
                .replace("{recipient}", recipient)
                .replace("{recipient.name}", recipient_name)
                .replace("{recipient.address}", &envelope.to.addr)
        };

        #[cfg(all(feature = "notify", target_os = "linux"))]
        if let Some(notify) = hook.notify.as_ref() {
            let res = Notification::new()
                .summary(&replace(&notify.summary, envelope))
                .body(&replace(&notify.body, envelope))
                .show_async()
                .await;
            if let Err(err) = res {
                debug!("error while sending system notification");
                debug!("{err:?}");
            }
        }

        #[cfg(all(feature = "notify", not(target_os = "linux")))]
        if let Some(notify) = hook.notify.as_ref() {
            let summary = replace(&notify.summary, &envelope);
            let body = replace(&notify.body, &envelope);

            let res = tokio::task::spawn_blocking(move || {
                Notification::new().summary(&summary).body(&body).show()
            })
            .await;

            if let Err(err) = res {
                debug!("cannot send system notification");
                debug!("{err:?}");
            } else {
                let res = res.unwrap();
                if let Err(err) = res {
                    debug!("error while sending system notification");
                    debug!("{err:?}");
                }
            }
        }

        if let Some(callback) = hook.callback.as_ref() {
            let res = callback(envelope).await;
            if let Err(_err) = res {
                debug!("error while executing callback");
                debug!("{_err:?}");
            }
        }
    }

    /// Find the alias of the given folder name.
    ///
    /// The alias is also shell expanded.
    pub fn find_folder_alias(&self, from_name: &str) -> Option<String> {
        self.folder
            .as_ref()
            .and_then(|c| c.aliases.as_ref())
            .and_then(|aliases| {
                aliases.iter().find_map(|(name, alias)| {
                    if name.eq_ignore_ascii_case(from_name.trim()) {
                        Some(shellexpand_str(alias))
                    } else {
                        None
                    }
                })
            })
    }

    /// Find the alias of the given folder, otherwise return the given
    /// folder itself.
    pub fn get_folder_alias(&self, folder: &str) -> String {
        self.find_folder_alias(folder)
            .unwrap_or_else(|| shellexpand_str(folder))
    }

    /// Get the inbox folder alias.
    pub fn get_inbox_folder_alias(&self) -> String {
        self.get_folder_alias(INBOX)
    }

    /// Get the sent folder alias.
    pub fn get_sent_folder_alias(&self) -> String {
        self.get_folder_alias(SENT)
    }

    /// Get the drafts folder alias.
    pub fn get_drafts_folder_alias(&self) -> String {
        self.get_folder_alias(DRAFTS)
    }

    /// Get the trash folder alias.
    pub fn get_trash_folder_alias(&self) -> String {
        self.get_folder_alias(TRASH)
    }

    /// Return `true` if the given folder matches the Trash folder.
    pub fn is_trash_folder(&self, folder: &str) -> bool {
        self.get_folder_alias(folder) == self.get_trash_folder_alias()
    }

    /// Return `true` if the delete message style matches the
    /// flag-based message deletion style.
    pub fn is_delete_message_style_flag(&self) -> bool {
        self.message
            .as_ref()
            .and_then(|c| c.delete.as_ref())
            .and_then(|c| c.style.as_ref())
            .filter(|c| c.is_flag())
            .is_some()
    }

    /// Get all folder aliases.
    pub fn get_folder_aliases(&self) -> Option<&HashMap<String, String>> {
        self.folder.as_ref().and_then(|c| c.aliases.as_ref())
    }

    /// Find the folder kind associated to the given folder alias.
    ///
    /// This function is the reverse of [`get_folder_alias`], as it
    /// tries to find a key (folder kind) matching the given value
    /// (folder alias).
    pub fn find_folder_kind_from_alias(&self, alias: &str) -> Option<FolderKind> {
        self.folder
            .as_ref()
            .and_then(|c| c.aliases.as_ref())
            .and_then(|aliases| {
                let from_alias = shellexpand_str(alias);
                aliases.iter().find_map(|(kind_or_name, alias)| {
                    if shellexpand_str(alias).eq_ignore_ascii_case(&from_alias) {
                        Some(kind_or_name.into())
                    } else {
                        None
                    }
                })
            })
    }

    /// Get the envelope listing page size if defined, otherwise
    /// return the default one.
    pub fn get_envelope_list_page_size(&self) -> usize {
        self.envelope
            .as_ref()
            .and_then(|c| c.list.as_ref())
            .and_then(|c| c.page_size)
            .unwrap_or(DEFAULT_PAGE_SIZE)
    }

    /// Get the envelope threading page size if defined, otherwise
    /// return the default one.
    #[cfg(feature = "thread")]
    pub fn get_envelope_thread_page_size(&self) -> usize {
        self.envelope
            .as_ref()
            .and_then(|c| c.thread.as_ref())
            .and_then(|c| c.page_size)
            .unwrap_or(DEFAULT_PAGE_SIZE)
    }

    /// Get the message reading format if defined, otherwise return
    /// the default one.
    pub fn get_message_read_format(&self) -> EmailTextPlainFormat {
        self.message
            .as_ref()
            .and_then(|c| c.read.as_ref())
            .and_then(|c| c.format.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    /// Get the message reading headers if defined, otherwise return
    /// the default ones.
    pub fn get_message_read_headers(&self) -> Vec<String> {
        self.message
            .as_ref()
            .and_then(|c| c.read.as_ref())
            .and_then(|c| c.headers.as_ref())
            .cloned()
            .unwrap_or(vec![
                "From".into(),
                "To".into(),
                "Cc".into(),
                "Subject".into(),
            ])
    }

    /// Get the message writing headers if defined, otherwise return
    /// the default ones.
    pub fn get_message_write_headers(&self) -> Vec<String> {
        self.message
            .as_ref()
            .and_then(|c| c.write.as_ref())
            .and_then(|c| c.headers.as_ref())
            .cloned()
            .unwrap_or(vec![
                "From".into(),
                "To".into(),
                "In-Reply-To".into(),
                "Cc".into(),
                "Subject".into(),
            ])
    }

    /// Find the message pre-send hook.
    pub fn find_message_pre_send_hook(&self) -> Option<&Command> {
        self.message
            .as_ref()
            .and_then(|c| c.send.as_ref())
            .and_then(|c| c.pre_hook.as_ref())
    }

    /// Return `true` if a copy of sent messages should be saved in
    /// the sent folder.
    pub fn should_save_copy_sent_message(&self) -> bool {
        self.message
            .as_ref()
            .and_then(|c| c.send.as_ref())
            .and_then(|c| c.save_copy)
            .unwrap_or(true)
    }

    /// Generate a template interpreter with prefilled options from
    /// the current user account configuration.
    pub fn generate_tpl_interpreter(&self) -> MimeInterpreterBuilder {
        let builder =
            MimeInterpreterBuilder::new().with_save_attachments_dir(self.get_downloads_dir());

        #[cfg(feature = "pgp")]
        if let Some(ref pgp) = self.pgp {
            return builder.with_pgp(pgp.clone());
        }

        builder
    }

    /// Get the envelope listing datetime format, otherwise return the
    /// default one.
    pub fn get_envelope_list_datetime_fmt(&self) -> String {
        self.envelope
            .as_ref()
            .and_then(|c| c.list.as_ref())
            .and_then(|c| c.datetime_fmt.clone())
            .unwrap_or_else(|| String::from("%F %R%:z"))
    }

    /// Return `true` if the envelope listing datetime local timezone
    /// option is enabled.
    pub fn has_envelope_list_datetime_local_tz(&self) -> bool {
        self.envelope
            .as_ref()
            .and_then(|c| c.list.as_ref())
            .and_then(|c| c.datetime_local_tz)
            .unwrap_or_default()
    }

    /// Get the new template signature placement.
    pub fn get_new_template_signature_style(&self) -> NewTemplateSignatureStyle {
        self.template
            .as_ref()
            .and_then(|c| c.new.as_ref())
            .and_then(|c| c.signature_style.clone())
            .unwrap_or_default()
    }

    pub fn get_reply_template_signature_style(&self) -> ReplyTemplateSignatureStyle {
        self.template
            .as_ref()
            .and_then(|c| c.reply.as_ref())
            .and_then(|c| c.signature_style.clone())
            .unwrap_or_default()
    }

    pub fn get_reply_template_posting_style(&self) -> ReplyTemplatePostingStyle {
        self.template
            .as_ref()
            .and_then(|c| c.reply.as_ref())
            .and_then(|c| c.posting_style.clone())
            .unwrap_or_default()
    }

    pub fn get_reply_template_quote_headline(&self, msg: &mail_parser::Message) -> Option<String> {
        let date = from_mail_parser_to_chrono_datetime(msg.date()?)?;

        let senders = match (msg.from(), msg.sender()) {
            (Some(List(a)), _) if !a.is_empty() => {
                a.iter().fold(String::new(), |mut senders, sender| {
                    if let Some(name) = sender.name() {
                        if !senders.is_empty() {
                            senders.push_str(", ");
                        }
                        senders.push_str(name);
                    } else if let Some(addr) = sender.address() {
                        if !senders.is_empty() {
                            senders.push_str(", ");
                        }
                        senders.push_str(addr);
                    }
                    senders
                })
            }
            (Some(Group(g)), _) if !g.is_empty() => {
                g.iter().fold(String::new(), |mut senders, sender| {
                    if let Some(ref name) = sender.name {
                        if !senders.is_empty() {
                            senders.push_str(", ");
                        }
                        senders.push_str(name);
                    }
                    senders
                })
            }
            (_, Some(List(a))) if !a.is_empty() => {
                a.iter().fold(String::new(), |mut senders, sender| {
                    if let Some(name) = sender.name() {
                        if !senders.is_empty() {
                            senders.push_str(", ");
                        }
                        senders.push_str(name);
                    } else if let Some(addr) = sender.address() {
                        if !senders.is_empty() {
                            senders.push_str(", ");
                        }
                        senders.push_str(addr);
                    }
                    senders
                })
            }
            (_, Some(Group(g))) if !g.is_empty() => {
                g.iter().fold(String::new(), |mut senders, sender| {
                    if let Some(ref name) = sender.name {
                        if !senders.is_empty() {
                            senders.push_str(", ");
                        }
                        senders.push_str(name);
                    }
                    senders
                })
            }
            _ => String::new(),
        };

        let fmt = self
            .template
            .as_ref()
            .and_then(|c| c.reply.as_ref())
            .and_then(|c| c.quote_headline_fmt.clone())
            .unwrap_or_else(|| String::from("On %d/%m/%Y %H:%M, {senders} wrote:\n"));

        Some(date.format(&fmt.replace("{senders}", &senders)).to_string())
    }

    pub fn get_forward_template_signature_style(&self) -> ForwardTemplateSignatureStyle {
        self.template
            .as_ref()
            .and_then(|c| c.forward.as_ref())
            .and_then(|c| c.signature_style.clone())
            .unwrap_or_default()
    }

    pub fn get_forward_template_posting_style(&self) -> ForwardTemplatePostingStyle {
        self.template
            .as_ref()
            .and_then(|c| c.forward.as_ref())
            .and_then(|c| c.posting_style.clone())
            .unwrap_or_default()
    }

    pub fn get_forward_template_quote_headline(&self) -> String {
        self.template
            .as_ref()
            .and_then(|c| c.forward.as_ref())
            .and_then(|c| c.quote_headline.clone())
            .unwrap_or_else(|| String::from("-------- Forwarded Message --------\n"))
    }
}

impl<'a> From<&'a AccountConfig> for Address<'a> {
    fn from(config: &'a AccountConfig) -> Self {
        Address::Address(EmailAddress {
            name: config.display_name.as_ref().map(Into::into),
            email: config.email.as_str().into(),
        })
    }
}

/// Rename duplicated file by adding a auto-incremented counter
/// suffix.
///
/// Helper that check if the given file path already exists: if so,
/// creates a new path with an auto-incremented integer suffix and
/// returs it, otherwise returs the original file path.
pub(crate) fn rename_file_if_duplicate(
    origin_file_path: &Path,
    is_file: impl Fn(&PathBuf, u8) -> bool,
) -> Result<PathBuf> {
    let mut count = 0;

    let mut file_path = origin_file_path.to_owned();
    let file_stem = origin_file_path.file_stem().and_then(OsStr::to_str);
    let file_ext = origin_file_path
        .extension()
        .and_then(OsStr::to_str)
        .map(|fext| String::from(".") + fext)
        .unwrap_or_default();

    while is_file(&file_path, count) {
        count += 1;
        file_path.set_file_name(
            &file_stem
                .map(|fstem| format!("{}_{}{}", fstem, count, file_ext))
                .ok_or_else(|| Error::ParseDownloadFileNameError(file_path.to_owned()))?,
        );
    }

    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn rename_file_if_duplicate() {
        let path = PathBuf::from("downloads/file.ext");

        // when file path is unique
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, _| false),
            Ok(path) if path == PathBuf::from("downloads/file.ext")
        ));

        // when 1 file path already exist
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count <  1),
            Ok(path) if path == PathBuf::from("downloads/file_1.ext")
        ));

        // when 5 file paths already exist
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5.ext")
        ));

        // when file path has no extension
        let path = PathBuf::from("downloads/file");
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5")
        ));

        // when file path has 2 extensions
        let path = PathBuf::from("downloads/file.ext.ext2");
        assert!(matches!(
            super::rename_file_if_duplicate(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file.ext_5.ext2")
        ));
    }
}
