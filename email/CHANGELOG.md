# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.26.4] - 2025-01-11

### Changed

- Bumped `imap-client@0.2.3`.

## [0.26.3] - 2025-01-11

### Changed

- Changed default `MessageSendConfig::save_copy` to `true`. [himalaya#536]

### Fixed

- Fixed list envelopes out of bound error when empty result. [himalaya#535]

## [0.26.2] - 2024-12-09

### Changed

- Bumped `oauth-lib` to `v2.0.0`.

## [0.26.1] - 2024-12-09

### Fixed

- Fixed MML markup shown in reply threads. [himalaya#487]

## [0.26.0] - 2024-10-28

### Added

- Added `tokio-rustls` cargo feature. The aim is to be able, one day, to compile `email-lib` with other async runtimes and other TLS options.

### Changed

- Removed `serde::flatten` from `ImapConfig::auth` and `SmtpConfig::auth`.
- Added `serde::tag = "type"` to `ImapAuthConfig` and `SmtpAuthConfig`.
- Added `OAuth2Config::redirect_{scheme,host,port}`.
- Added new boolean `Envelope::has_attachment` to determine if an envelope has at least one attachment.
- Added `ImapConfig::extensions` of type `Option<ImapExtensionsConfig>`.
- Changed the way `ID` command is automatically sent after authentication. The `ID` command is now sent if and only if `ImapConfig.extensions.id.send_after_auth` is `true`. See [#25](https://github.com/modern-email/defects/issues/25) for more information.
- Renamed `Pgp` variants according to `mml` module.

## [0.25.0] - 2024-08-16

### Added

- Added `keyring` cargo feature, which enables the global user keyring system for storing and retrieving secrets.
- Added `oauth2` cargo feature, which enables OAuth 2.0 support.
- Added `notify` cargo feature, which enables watch system notification action.
- Added `pool` cargo feature, which enables the `thread_pool` module.
- Added `watch` cargo feature, which enables watch features (only `WatchEnvelopes` is available at the moment).

### Changed

- Replaced `imap` crate by `imap-{types,codec,next,client}` suite.
- Replaced `maildirpp` crate by `maildirs`, which improves Maildir supports.
- Renamed `account-sync` cargo feature by `sync`.
- Renamed `account-discovery` cargo feature by `autoconfig`.
- Moved `account::discover` module at the root level, and renamed it `autoconfig`.
- Bumped autoconfig `hyper` to `v1`.

### Fixed

- Made reply template headers more reliable.

  When replying to a message, the `Reply-To` is used as recipients if existing, otherwise it uses all recipients from `From` (or `Sender` if missing) and `To`, minus yourself and noreply addresses. When replying all to a message, the `Cc` is also used minus yourself and noreply addresses.

## [0.24.1] - 2024-04-16

### Fixed

- Fixed error page out of bounds when filtering envelopes returned an empty result [#195].

## [0.24.0] - 2024-04-14

### Added

- Added `SyncBuilder` setters to customize folder filters and permissions, envelope filters, flag permissions and message permissions.

### Changed

- Renamed `SyncBuilder::{set,set_some,with,with_some}_folders_filter` to `*_folder_filters` to match other namings.
- Applied sync filters at patch creation rather than patch execution.
- Envelope sync date filters `before` and `after` takes now a `NaiveDate` instead, to match the search envelope query.

## [0.23.2] - 2024-04-09

### Changed

- Bumped `mml@1.0.12`.

### Removed

- Removed `chumsky@1.0.0-alpha.7`'s `spill-stack` feature due to cross-compilation issues.

## [0.23.1] - 2024-04-08

### Changed

- Bumped `keyring@0.4.2`.
- Bumped `secret@0.4.4`.
- Bumped `mml@1.0.11`.

## [0.23.0] - 2024-04-08

### Added

- Added cargo feature `derive` to enable/disable (de)serialization of structs using `serde`.
- Added workspace `email-testing-server` containing code to spawn an email testing server (IMAP, JMAP and SMTP), based on [mail-server](https://github.com/stalwartlabs/mail-server/blob/main/crates/main/src/main.rs).
- Added new trait `HasAccountConfig` used by backend features.
- Added module `email::search_query` containing the `SearchEmailsQuery` struct and parsers. See the API documentation for more details on the search query.
- Added setters to customize `SyncBuilder` pool size: `set_pool_size`, `set_some_pool_size`, `with_pool_size`, `with_some_pool_size`.
- Added setters to customize `ThreadPoolBuilder` pool size: `set_size`, `set_some_size`, `with_size`, `with_some_size`.
- Added `template::Template` that holds the template content and the cursor position.
- Added `template::new::NewTemplateSignatureStyle`.
- Added `template::reply::{ReplyTemplateSignatureStyle, ReplyTemplatePostingStyle}`.
- Added `template::forward::{ForwardTemplateSignatureStyle, ForwardTemplatePostingStyle}`.
- Added `AccountConfig::get_new_template_signature_style() -> NewTemplateSignatureStyle`.
- Added `AccountConfig::get_reply_template_signature_style() -> ReplyTemplateSignatureStyle`.
- Added `AccountConfig::get_reply_template_posting_style() -> ReplyTemplatePostingStyle`.
- Added `AccountConfig::get_reply_template_quote_headline() -> Option<String>`.
- Added `AccountConfig::get_forward_template_signature_style() -> ForwardTemplateSignatureStyle`.
- Added `AccountConfig::get_forward_template_posting_style() -> ForwardTemplatePostingStyle`.
- Added `AccountConfig::get_forward_template_quote_headline() -> Option<String>`.
- Added `MessageConfig::delete` of type `Option<DeleteMessageConfig>` in order to configure deletion style (folder-based or flag-based) [#169].

### Changed

- Refactored the whole error system, see <https://docs.rs/email-lib/0.23.0/email/>.
- `ListEnvelopes` takes now a `ListEnvelopesOptions` composed of `page: usize`, `page_size: usize` and `query: Option<SearchEmailsQuery>`.
- Changed `AccountConfig::find_full_signature` signature to `Option<String>` (removed unused `Result`).
- Renamed `NewTplBuilder` into `NewTemplateBuilder`.
- Renamed `ReplyTplBuilder` into `ReplyTemplateBuilder`.
- Renamed `ForwardTplBuilder` into `ForwardTemplateBuilder`.
- Changed return type of `{New,Reply,Forward}TemplateBuilder::build`: they now return the new `Template` struct that holds the template content and the cursor position.
- Changed `config` param type of `{New,Reply,Forward}TemplateBuilder::new`: they take now a `Arc<AccountConfig>` instead of `&AccountConfig`.
- Added `set_posting_style`, `set_some_posting_style`, `with_posting_style`, `with_some_posting_style` to `{Reply,Forward}TemplateBuilder::new`
- Added `set_signature_style`, `set_some_signature_style`, `with_signature_style`, `with_some_signature_style` to `{New,Reply,Forward}TemplateBuilder::new`
- Changed `DefaultDeleteMessages` behaviour: if the message deletion style matches the flag-based one, add the Deleted flag, otherwise move to Trash.
- Renamed `AccountSyncBuilder::new` to `try_new`.

### Fixed

- Fixed watch IMAP envelopes when folder was empty [#179].
- Fixed wrong recipient in reply template of mailing list messages [#187].
- Fixed timeout errors for IMAP and SMTP. Every action is now retried 3 times before aborting [#174].
- Fixed `serde` default sync permissions for folders, flags and messages.

### Removed

- Removed need for `docker` to run integration tests [#36].

## [0.22.3] - 2024-02-25

### Fixed

- Fixed watch notifications on MacOS and Windows.

## [0.22.2] - 2024-02-25

### Fixed

- Fixed `borrowed data escapes outside of method` error on MacOS and Windows.

## [0.22.1] - 2024-02-25

### Added

- Added backend builder feature `CheckUp` to check integrity of configuration and context.

## [0.22.0] - 2024-02-21

### Added

- Added function `AutoConfig::is_gmail`.
- Added `FolderConfig::sync` to customize folder sync.
- Added `FolderSyncConfig::filter` to filter sync folder by names.
- Added `FolderSyncConfig::permissions` to allow folder creation or deletion.
- Added `EnvelopeConfig::sync` to customize envelope sync.
- Added `EnvelopeSyncConfig::filter` to filter envelopes by date range.
- Added `FlagConfig::sync` to customize flag sync.
- Added `FlagSyncConfig::permissions` to allow flag update.
- Added `MessageConfig::sync` to customize message sync.
- Added `MessageSyncConfig::permissions` to allow message creating and deletion.

### Changed

- Changed `WatchHook` from enum to struct. This way, multiple hook variants can be set up for one event (like sending a system notification and executing a shell command when receiving a new envelope).
- Made function `DnsClient::get_mx_domain` public.
- Replaced the `AccountConfig::get_sync_dir` path from `$XDG_DATA_HOME/himalaya` to `$XDG_DATA_HOME/pimalaya/email/sync`. First, Himalaya should not be present in Pimalaya lib. Secondly the sync dir can now be shared between projects relying on `email-lib`.
- Replaced `SQLite` sync cache by a light version of the `Maildir` backend, where only `Message-ID` and `Date` headers from messages are kept.
- Refactored the whole sync system: the backend sync is now generic (it can sync 2 different backends together) and its code has been extracted into a dedicated module `sync`. The sync patch applier (which used to process hunk in parallel) is now generic and its code has been extracted into a dedicated module `thread_pool` (it can execute generic tasks in parallel).
- Refactored the backend module: code has been splitted into submodules. The `Backend` struct became a trait `BackendFeatures`, which is just an alias for all features `AddFolder + ListEnvelopes + SendMessage + …`. The lib exposes two backend implementation: `Backend` (which is the direct equivalent of the previous struct) and `BackendPool` (which can execute features in parallel).
- Moved `account::SyncConfig::strategy` to `FolderConfig::sync` > `FolderSyncConfig::filter`.

### Fixed

- Fixed watch IMAP envelopes issue preventing events to be triggered.
- Fixed MX DNS account discovery that was not checking the ISPDB before checking TXT records.
- Fixed SMTP messages not properly sent to all recipients [#172].

### Removed

- Removed function `DnsClient::get_mailconf_mx_uri`.
- Removed cargo features `folder`, `account`, `flag`, `message`, and all associated sub features. The code started to be too hard to maintain. Adding so many features was a wrong choice.
- Removed `AccountConfig::get_sync_db_conn` function.

## [0.21.0] - 2024-01-27

### Added

- Added `account::discover` module to help lib consumers to detect automatically IMAP and SMTP settings.
- Added Notmuch contexts and context builder.
- Added Notmuch backend features `AddNotmuchFolder`, `ListNotmuchFolders`, `GetNotmuchEnvelope`, `ListNotmuchEnvelopes`, `AddNotmuchFlags`, `SetNotmuchFlags`, `RemoveNotmuchFlags`, `AddNotmuchMessage`, `PeekNotmuchMessages`, `CopyNotmuchMessages` and `MoveNotmuchMessages`.
- Added `NotmuchConfig::maildir_path` of type `Option<PathBuf>` to customize the path to the Maildir folder. Defaults to `NotmuchConfig::database_path`.
- Added `NotmuchConfig::config_path` of type `Option<PathBuf>` to customize the path to the Notmuch configuration file.
- Added `NotmuchConfig::profile` of type `Option<String>` to customize the Notmuch profile to use.

### Changed

- Renamed cargo feature `sync` to `account-sync`.
- Added variant `WatchHook::Fn` that takes a `WatchFn` as argument. A `WatchFn` is just a wrapper around a `Fn(&Envelope) -> Result<()>`.
- Renamed `MaildirSessionSync::session` by `MaildirContextSync::inner`.
- Renamed `MaildirSession` and `MaildirSessionSync` by `MaildirContext` and `MaildirContextSync`.
- Renamed `ImapSessionSync::session` by `ImapContextSync::inner`.
- Renamed `ImapSession` and `ImapSessionSync` by `ImapContext` and `ImapContextSync`.
- Renamed `AddFolderMaildir` by `AddMaildirFolder`.
- Replaced `AddMaildirFolder::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn AddFolder>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn AddFolder>>`
- Renamed `AddFolderImap` by `AddImapFolder`.
- Replaced `AddImapFolder::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn AddFolder>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn AddFolder>>`
- Renamed `ListFoldersMaildir` by `ListMaildirFolders`.
- Replaced `ListMaildirFolders::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn ListFolders>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ListFolders>>`
- Renamed `ListFoldersImap` by `ListImapFolders`.
- Replaced `ListImapFolders::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn ListFolders>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ListFolders>>`
- Renamed `ExpungeFolderMaildir` by `ExpungeMaildirFolder`.
- Replaced `ExpungeMaildirFolder::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn ExpungeFolder>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ExpungeFolder>>`
- Renamed `ExpungeFolderImap` by `ExpungeImapFolder`.
- Replaced `ExpungeImapFolder::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn ExpungeFolder>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ExpungeFolder>>`
- Renamed `PurgeFolderImap` by `PurgeImapFolder`.
- Replaced `PurgeImapFolder::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn PurgeFolder>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn PurgeFolder>>`
- Renamed `DeleteFolderMaildir` by `DeleteMaildirFolder`.
- Replaced `DeleteMaildirFolder::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn DeleteFolder>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn DeleteFolder>>`
- Renamed `DeleteFolderImap` by `DeleteImapFolder`.
- Replaced `DeleteImapFolder::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn DeleteFolder>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn DeleteFolder>>`
- Renamed `GetEnvelopeMaildir` by `GetMaildirEnvelope`.
- Replaced `GetMaildirEnvelope::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn GetEnvelope>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn GetEnvelope>>`
- Renamed `GetEnvelopeImap` by `GetImapEnvelope`.
- Replaced `GetImapEnvelope::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn GetEnvelope>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn GetEnvelope>>`
- Renamed `ListEnvelopesMaildir` by `ListMaildirEnvelopes`.
- Replaced `ListMaildirEnvelopes::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn ListEnvelopes>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ListEnvelopes>>`
- Renamed `ListEnvelopesImap` by `ListImapEnvelopes`.
- Replaced `ListImapEnvelopes::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn ListEnvelopes>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ListEnvelopes>>`
- Replaced `WatchMaildirEnvelopes::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn WatchEnvelopes>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn WatchEnvelopes>>`
- Replaced `WatchImapEnvelopes::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn WatchEnvelopes>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn WatchEnvelopes>>`
- Renamed `AddFlagsMaildir` by `AddMaildirFlags`.
- Replaced `AddMaildirFlags::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn AddFlags>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn AddFlags>>`
- Renamed `AddFlagsImap` by `AddImapFlags`.
- Replaced `AddImapFlags::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn AddFlags>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn AddFlags>>`
- Renamed `SetFlagsMaildir` by `SetMaildirFlags`.
- Replaced `SetMaildirFlags::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn SetFlags>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn SetFlags>>`
- Renamed `SetFlagsImap` by `SetImapFlags`.
- Replaced `SetImapFlags::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn SetFlags>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn SetFlags>>`
- Renamed `RemoveFlagsMaildir` by `RemoveMaildirFlags`.
- Replaced `RemoveMaildirFlags::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn RemoveFlags>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn RemoveFlags>>`
- Renamed `RemoveFlagsImap` by `RemoveImapFlags`.
- Replaced `RemoveImapFlags::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn RemoveFlags>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn RemoveFlags>>`
- Replaced `AddMaildirMessage::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn AddMessage>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn AddMessage>>`
- Replaced `AddImapMessage::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn AddMessage>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn AddMessage>>`
- Renamed `PeekMessagesMaildir` by `PeekMaildirMessages`.
- Replaced `PeekMaildirMessages::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn PeekMessages>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn PeekMessages>>`
- Renamed `PeekMessagesImap` by `PeekImapMessages`.
- Replaced `PeekImapMessages::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn PeekMessages>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn PeekMessages>>`
- Renamed `GetMessagesMaildir` by `GetMaildirMessages`.
- Replaced `GetMaildirMessages::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn GetMessages>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn GetMessages>>`
- Renamed `GetMessagesImap` by `GetImapMessages`.
- Replaced `GetImapMessages::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn GetMessages>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn GetMessages>>`
- Renamed `CopyMessagesMaildir` by `CopyMaildirMessages`.
- Replaced `CopyMaildirMessages::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn CopyMessages>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn CopyMessages>>`
- Renamed `CopyMessagesImap` by `CopyImapMessages`.
- Replaced `CopyImapMessages::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn CopyMessages>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn CopyMessages>>`
- Renamed `MoveMessagesMaildir` by `MoveMaildirMessages`.
- Replaced `MoveMaildirMessages::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn MoveMessages>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn MoveMessages>>`
- Renamed `MoveMessagesImap` by `MoveImapMessages`.
- Replaced `MoveImapMessages::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn MoveMessages>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn MoveMessages>>`
- Renamed `DeleteMessagesMaildir` by `DeleteMaildirMessages`.
- Replaced `DeleteMaildirMessages::new` by:
  - `new(ctx: &MaildirContextSync) -> Self`
  - `new_boxed(ctx: &MaildirContextSync) -> Box<dyn DeleteMessages>`
  - `some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn DeleteMessages>>`
- Renamed `DeleteMessagesImap` by `DeleteImapMessages`.
- Replaced `DeleteImapMessages::new` by:
  - `new(ctx: &ImapContextSync) -> Self`
  - `new_boxed(ctx: &ImapContextSync) -> Box<dyn DeleteMessages>`
  - `some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn DeleteMessages>>`
- Renamed `SendMessageSendmail` by `SendSendmailMessage`.
- Replaced `SendSendmailMessage::new` by:
  - `new(ctx: &SendmailContextSync) -> Self`
  - `new_boxed(ctx: &SendmailContextSync) -> Box<dyn SendMessage>`
  - `some_new_boxed(ctx: &SendmailContextSync) -> Option<Box<dyn SendMessage>>`
- Renamed `SendMessageSmtp` by `SendSmtpMessage`.
- Replaced `SendSmtpMessage::new` by:
  - `new(ctx: &SmtpContextSync) -> Self`
  - `new_boxed(ctx: &SmtpContextSync) -> Box<dyn SendMessage>`
  - `some_new_boxed(ctx: &SmtpContextSync) -> Option<Box<dyn SendMessage>>`
- Changed `NotmuchConfig::db_path: PathBuf` by `database_path: Option<PathBuf>`. A serde alias is available.
- Moved `AccountConfig` parameter from `{any}ContextBuilder::new` to `BackendContextBuilder::build`

## [0.20.1] - 2024-01-12

### Fixed

- Fixed de/serialization issues with commands and OAuth 2.0.

## [0.20.0] - 2024-01-08

### Added

- Added cargo feature `sync`.
- Added one cargo feature per backend feature:
  - `folder` including `folder-add`, `folder-list`, `folder-expunge`, `folder-purge`, `folder-delete`
  - `envelope` including `envelope-list`, `envelope-watch`, `envelope-get`
  - `flag` including `flag-add`, `flag-set`, `flag-remove`
  - `message` including `message-add`, `message-peek`, `message-get`, `message-copy`, `message-move`, `message-delete`, `message-send`.

### Changed

- Merged `AddRawMessage` and `AddRawMessageWithFlags` into one single trait `AddMessage` that exposes 3 functions: `add_message_with_flags` (implem required), `add_message_with_flag` and `add_message`.

## [0.19.6] - 2024-01-06

### Added

- Added cargo feature `sync` to enable (default) or disable account synchronization.

### Fixed

- Fixed builds errors related to cargo features.

## [0.19.5] - 2024-01-01

### Fixed

- Adjusted watch notifications on all platforms.

## [0.19.4] - 2023-12-31

### Fixed

- Fixed watch notifications on MacOS.

## [0.19.3] - 2023-12-31

### Fixed

- Fixed watch notifications on Windows and Linux.

## [0.19.2] - 2023-12-31

### Changed

- Bumped `keyring-lib@0.3.2`.
- Bumped `secret-lib@0.3.2`.
- Bumped `mml-lib@1.0.6`.

## [0.19.1] - 2023-12-31

### Changed

- Bumped `keyring-lib@0.3.1`.
- Bumped `secret-lib@0.3.1`.
- Bumped `mml-lib@1.0.5`.

## [0.19.0] - 2023-12-31

### Added

- Added option `ImapConfig::watch` of type `ImapWatchConfig`.
- Added option `ImapWatchConfig::timeout` to customize the default IMAP IDLE timeout of 29 mins as defined in the RFC.

### Changed

- Replaced options `ImapConfig::ssl`, `ImapConfig::starttls` and `ImapConfig::insecure` by `ImapConfig::encryption`:
  - `ImapEncryptionConfig::Tls`: use required encryption (SSL/TLS)
  - `ImapEncryptionConfig::StartTls`: use opportunistic encryption (StartTLS)
  - `ImapEncryptionConfig::None`: do not use any encryption
- Replaced options `SmtpConfig::ssl`, `SmtpConfig::starttls` and `SmtpConfig::insecure` by `SmtpConfig::encryption`:
  - `SmtpEncryptionConfig::Tls`: use required encryption (SSL/TLS)
  - `SmtpEncryptionConfig::StartTls`: use opportunistic encryption (StartTLS)
  - `SmtpEncryptionConfig::None`: do not use any encryption

### Removed

- Removed unused `ImapConfig::notify_cmd`.
- Removed unused `ImapConfig::notify_query`.
- Removed unused `ImapConfig::watch_cmds`.

## [0.18.5] - 2023-12-24

### Changed

- Bumped `imap` from `3.0.0-alpha.10` to `3.0.0-alpha.12`.
- Bumped `imap-proto` from `0.16.2` to `0.16.3`.

## [0.18.4] - 2023-12-20

### Added

- Added `Backend::send_reply_raw_message` that sends a raw message and apply the Answered flag if the `add_flags` feature is available.

## [0.18.3] - 2023-12-20

### Fixed

- Added missing serde `rename_all`.

## [0.18.2] - 2023-12-20

### Fixed

- Fixed `Backend::send_raw_message` that was not saving copy of sent message to the Sent folder.

## [0.18.1] - 2023-12-19

### Changed

- Bumped `mml@1.0.4`.

## [0.18.0] - 2023-12-16

### Added

- Added backend feature `WatchEnvelopes` that triggers hooks defined in the new account configuration entry `EnvelopeConfig::watch` at path `envelope.watch.{event}.{hook}`.
- Added 2 envelope watch events `WatchEnvelopeConfig::received` (when a new envelope is detected) and `WatchEnvelopeConfig::any` (for all other cases, the fallback).
- Added 2 watch hooks `WatchHook::Cmd` (execute a shell command) and `WatchHook::Notify(WatchNotifyConfig)` (send a system notification). The last one takes a summary and a body to customize the final notification.
- Added `FolderKind` enum to categorize a folder (inbox, sent, drafts, trash).
- Added `AccountConfig::find_folder_kind_from_alias` function to find the folder kind associated to a given folder alias.

### Changed

- Changed `AccountConfig::{get,find}_folder_alias` return type: they do not return a `Result` anymore.
- Added `Folder::kind` of type `Option<FolderKind>`.
- Changed the synchronization algorithm related to folders: it uses now the folder kind instead of the folder name to compare and store cache lines.

## [0.17.1] - 2023-12-11

### Fixed

- Fixed serde config for `PgpConfig` struct.

## [0.17.0] - 2023-12-11

### Changed

- Refactored `Config` and `AccountConfig`: instead of having flat levels like `email_listing_page_size`, there is now folded levels `envelope.list.page_size`. Added root levels `folder`, `envelope`, `message`. See the structs for details.
- Made all structures and enums related to config serializable and deserializable using `serde`.

## [0.16.0] - 2023-12-10

### Added

- Added `Config` structure that represents the global settings of the user, including all his accounts in a `HashMap<String, AccountConfig>` [#110].
- Added new cargo features related to backend features: `maildir` and `sendmail`.

### Changed

- Replaced `Backend` and `Sender` traits by small backend feature traits [#103].
- Renamed backend and sender cargo features:
  - `imap-backend` => `imap`
  - `notmuch-backend` => `notmuch`
  - `smtp-sender` => `smtp`
- Bumped all inner crates.

### Fixed

- Fixed prefixes not set properly in reply and forward templates. Now prefixes are identified using `Regex` and removed, which should avoid multiple nested prefixes like `Re: RE:Hello, world!`.
- Fixed a bug when an attachment could be downloaded outside of the downloads directory [#158].
- Fixed `AccountConfig::sync_dir` not expanded properly [#152].

## [0.15.3] - 2023-09-25

### Changed

- Bumped `mml@0.5.0`.

### Fixed

- Fixed integration tests.

## [0.15.2] - 2023-08-29

### Changed

- Renamed `pimalaya-shellexpand` to `shellexpand-utils`.

## [0.15.1] - 2023-08-29

### Changed

- Replaced `shellexpand` by `pimalaya-shellexpand`.

## [0.15.0] - 2023-08-27

### Added

- Added 3 new cargo features:
  - `pgp-commands`: enables the commands PGP backend (enabled by default, same behaviour as before)
  - `pgp-gpg`: enables the GPG backend (requires the `gpgme` lib on the system)
  - `pgp-native`: enables the native PGP backend
- Added `AccountConfig::pgp` of type `PgpConfig`.

### Changed

- Renamed project `email-lib` in order to make it generic.

### Fixed

- Fixed first time reading message not working [#97].

### Removed

- Removed `AccountConfig::email_writing_encrypt_cmd`.
- Removed `AccountConfig::email_reading_decrypt_cmd`.
- Removed `AccountConfig::email_writing_sign_cmd`.
- Removed `AccountConfig::email_reading_verify_cmd`.

## [0.14.0] - 2023-07-18

### Changed

- Changed the way folder aliases are resolved. They are now resolved directly from backend implementations, which frees interfaces from this responsibility [#95].
- Bumped `pimalaya_email_tpl@0.3.1`.

### Fixed

- Fixed absolute folder aliases for the maildir backend [#94].
- Fixed notmuch virtual folder [#92].

## [0.13.0] - 2023-07-09

### Changed

- Made the code async. Functions from the traits `Backend` and `Sender` are also async using the `async_trait` crate.
- Bumped `pimalaya_secret@0.0.5`.

## [0.12.0] - 2023-06-29

### Changed

- Moved `backend::sync` module to `account::sync`, and renamed all associated structures `Backend*` by `Account*`.
- Replaced `From` and `Into` implementations for flags and envelopes by custom functions.
- Prefixed email message template builders (new, reply and forward) setters by `with_` to match other builders of the codebase.
- Refactored folders structure, see the new API at <https://docs.rs/pimalaya-email/0.12.0/pimalaya_email/>.

### Removed

- Flattened `/domain` folder.
- Removed `Folder::delim` field.

## [0.11.0] - 2023-06-15

### Added

- Added `AccountConfig::email_listing_datetime_fmt` to customize envelopes datetime format. See format spec at <https://docs.rs/chrono/latest/chrono/format/strftime/index.html>.
- Added `AccountConfig::email_listing_local_datetime` to transform envelopes datetime's timezone to the user's local one. For example, if the user's local is set to `UTC`, the envelope date `2023-06-15T09:00:00+02:00` becomes `2023-06-15T07:00:00-00:00`.

### Changed

- Changed `Envelope::date` from `DateTime<Local>` to `DateTime<FixedOffset>` in order to keep the original timezone. Timezone can be adjusted with the new option `AccountConfig::email_listing_local_datetime`.

### Fixed

- Fixed missing `<` and `>` around `Message-ID` and `In-Reply-To` headers.

## [0.10.0] - 2023-06-13

### Added

- Implemented OAuth 2.0 refresh token flow for IMAP and SMTP, which means that access tokens are now automatically refreshed.
- Added `OAuth2Config::redirect_host` and `OAuth2Config::redirect_port`, which means OAuth 2.0 redirect server host and port can be customized.

### Changed

- Changed `Backend` and `Sender` trait: functions now borrow `&self` as `mut`.
- Renamed `BackendSyncProgressEvent` events name.
- Renamed sync related structs by prefixing them with their domain. For example, `folder::sync::Cache` became `FolderSyncCache`.

### Removed

- Removed `ImapAuth`.
- Removed `Backend` derivations `Sync` and `Send`, because maintaining a session pool with `Mutex` was too much of a burden. Instead, backends can be duplicated using `BackendBuilder`. Behind the scene it just recreates a new backend with a new session.
- Removed `ImapBackendBuilder`.

## [0.9.0] - 2023-06-03

### Added

- Added IP support using `rustls` `v0.21` [#80].
- Added `AccountConfig::generate_tpl_interpreter` function to generate a template interpreter with default options based on the config (pgp encrypt, pgp verify and attachments dir).

### Changed

- Changed `AccountConfig::addr` return type from `lettre::Mailbox` to `mail_builder::Address`.
- Changed `AccountConfig::email_reading_headers` default values to `["From", "To", "Cc", "Subject"]`.
- Changed `AccountConfig::email_writing_headers` default values to `["From", "To", "In-Reply-To", "Cc", "Subject"]`.
- Removed noise around signature by trimming it.
- Changed `Email::parsed` return type from `mailparse::ParsedMail` to `mail_parser::Message`.
- Changed `Email::new_tpl_builder` return type from `Result<TplBuilder>` to `NewTplBuilder`.
- Renamed `Email::to_read_tpl_builder` to `Email::to_read_tpl` which returns now a `Result<Tpl>` directly.
- Changed `Email::to_reply_tpl_builder` return type from `Result<TplBuilder>` to `ReplyTplBuilder`.
- Changed `Email::to_forward_tpl_builder` return type from `Result<TplBuilder>` to `ForwardTplBuilder`.
- Renamed `backend::imap::Error::ListEnvelopesOutOfBounds` by `BuildPageRangeOutOfBoundsError`.
- Replaced [lettre] by [mail-send], [mailparse] by [mail-parser] and [maildir] by [maildirpp].
- Removed `native-tls` support, `rustls-tls` is now the only TLS provider available. Removed in consequence `native-tls`, `rustls-tls` and `rustls-native-certs` cargo features.

### Fixed

- Fixed notmuch path not being expanded correctly.
- Fixed `.notmuch` folder created by `notmuch new` command being treated as a folder. Because it is a folder starting by a dot, it was considered as a Maildir++ folder (which is not).
- Fixed IMAP pagination error when listing envelopes [#76].

## [0.8.0] - 2023-05-19

### Added

- Added OAuth 2.0 support for IMAP and SMTP [#9].
- Added [secret service] support via the [keyring] crate [#6].
- Added `ImapAuthConfig` struct that contains config related to OAuth 2.0. It also contains a `configure` method to set up client secret and store access token from redirect URL using [secret service].
- Added `AccountConfig::email_sending_save_copy` to save copy of sent email [#70].

### Changed

- Replaced `ImapConfig::passwd_cmd` with `ImapConfig::auth` which takes 2 variants:
  - `ImapAuthConfig::Passwd(PasswdConfig)` for password authentication
  - `ImapAuthConfig::OAuth2(OAuth2Config)` for OAuth 2.0 authentication
- Moved `backend::id_mapper` to the CLI crate.
- Renamed `EmailSender` to `Sender` and `AccountConfig::email_sender` to `AccountConfig::sender` in order to match `Backend`.
- Moved backend config to `AccountConfig::backend`. They do not need to be given separately to backend and sender builders.
- Changed `AccountConfig::*_cmd` from `String` to `pimalaya_process::Cmd`.

### Fixed

- Fixed synchronization deadlock due to IMAP watch commands not properly executed [#61].

### Removed

- Removed `rustls-native-certs` cargo feature, it is now included by default within the `rustls-tls` cargo feature.
- Removed `Backend::*_internal` functions, no more aliases are used within the lib [#38].

## [0.7.0] - 2023-05-01

### Added

- Initiated `.gitattributes` file [patch#4].
- Added new account option `sync_folders_strategy` which allows to choose a folders synchronization strategy:
  - `Strategy::All`: synchronize all existing folders
  - `Strategy::Include`: synchronize only the given folders
  - `Strategy::Exclude`: synchronizes all folders except the given ones
- Added warning message when `process::run` exit code is not `0` [patch#6].
- Added `vendored` feature (linked to the `native-tls/vendored` one).

### Changed

- Changed the way `Flag::Custom` is used: in order to have a more unified API across backends, the custom variant is only used when receiving data (not anymore when parsing data from backends). Therefore custom flags are not synchronized anymore (because custom flags are not supported by the Maildir backend).
- Returns an error if `BackendSyncBuilder::sync` cannot acquire the lock in order to avoid processes to block each other infinitely [patch#7].
- Made `rustls` the default feature over `native-tls` to improve compatibility among operating systems.

### Fixed

- Fixed date parsing using the [mail-parser] crate [#44].
- Fixed Cc addresses when replying all [#46].
- Clarified header/value trace logs [patch#2].
- Fixed default `imap-notify-cmd` placeholders not being replaced [patch#3].
- Fixed IMAP session pool errors [#50].
- Fixed wrong recipient when from = sender [#52].
- Fixed invalid `ProcessEnvelopesPatch` length [#57].
- Fixed the process.rs' `pipe` function so it returns exit code correctly [patch#5].
- Fixed notmuch folders management [#45].

### Removed

- Removed `serde::Serialize` trait from structures and `serde` deps.
- Removed variant `Recent` from flag.
- Removed `Flag::to_symbols_string`: the responsibility shifted client side.

## [0.6.0] - 2023-02-14

### Added

- Added ability to synchronize specific folders only [#37].
- Added `Backend::expunge` function that definitely removes emails with the `Deleted` flag.
- Added `Backend::mark_emails_as_deleted` function with a default implementation that adds the `Deleted` flag.

### Changed

- Changed the way emails are deleted. `Backend::delete_emails` now moves the email to the `Trash` folder (or to the corresponding alias from the config file). If the target folder is the `Trash` folder, it will instead add the `Deleted` flag. Emails are removed with the `Backend::expunge` function.

### Fixed

- Fixed `ImapBackend::list_envelopes` pagination.
- Fixed synchronization issues for emails without `Message-ID` header by using the `Date` header instead.
- Fixed maildir backend perfs issues by enabling the `mmap` feature of the `maildir` crate.
  
### Removed

- Removed the `maildir-backend` cargo feature, it is now included by default.

## [0.5.1] - 2023-02-08

### Fixed

- Fixed `notmuch` backend compilation error on rustc `v1.67+`.

## [0.5.0] - 2023-02-07

### Added

- Made backend functions accept a vector of id instead of a single id [#20].
- Added function `Backend::purge_folder` that removes all emails inside a folder.
- Added new `Backend` functions using the internal id:
  - `get_envelope_internal`: gets an envelope by its internal id
  - `add_email_internal`: adds an email and returns its internal id
  - `get_emails_internal`: gets emails by their internal id
  - `copy_emails_internal`: copies emails by their internal id
  - `move_emails_internal`: copies emails by their internal id
  - `delete_emails_internal`: copies emails by their internal id
  - `add_flags_internal`: adds emails flags by their internal id
  - `set_flags_internal`: set emails flags by their internal id
  - `remove_flags_internal`: removes emails flags by their internal id
- Added emails synchronization feature. Backends that implement the `ThreadSafeBackend` trait inherit the `sync` function that synchronizes all folders and emails with a local `Maildir` instance.
- Added `Backend::sync` function and link `ThreadSafeBackend::sync` to it for the IMAP and the Maildir backends.
- Added the ability to URL encode Maildir folders (in order to fix path collisions, for eg `[Gmail]/Sent`). Also added a `MaildirBackendBuilder` to facilitate the usage of the `url_encoded_folders` option.
- Added a process lock for `ThreadSafeBackend::sync`, this way only one synchronization can be performed at a time (for a same account).

### Fixed

- Used native IMAP commands `copy` and `mv`.
- Fixed maildir date envelope parsing.
- Fixed inline attachments not collected.

### Changed

- Improved `Backend` method names. Also replaced the `self mut` by a `RefCell`.
- Simplified the `Email` struct: there is no custom implementation with custom fields. Now, the `Email` struct is just a wrapper around `mailparse::ParsedMail`.
- Improved `Flag` structures.
- Changed `Backend` trait functions due to [#20]:
  - `list_envelope` => `list_envelopes`
  - `search_envelope` => `search_envelopes`
  - `get_email` => `get_emails`, takes now `ids: Vec<&str>` and returns an `Emails` structure instead of an `Email`
  - `copy_email` => `copy_emails`, takes now `ids: Vec<&str>`.
  - `move_email` => `move_emails`, takes now `ids: Vec<&str>`.
  - `delete_email` => `delete_emails`, takes now `ids: Vec<&str>`.
  - `add_flags` takes now `ids: Vec<&str>` and `flags: &Flags`.
  - `set_flags` takes now `ids: Vec<&str>` and `flags: &Flags`.
  - `remove_flags` takes now `ids: Vec<&str>` and `flags: &Flags`.

### Removed

- The `email::Tpl` structure moved to its [own repository](https://git.sr.ht/~soywod/mime-msg-builder).
- Encryption and signing moved with the `email::Tpl` in its own repository.

## [0.4.0] - 2022-10-12

### Added

- Added pipe support for `(imap|smtp)-passwd-cmd`.
- Added `imap-ssl` and `smtp-ssl` options to be able to disable encryption.
- Implemented sendmail sender.
- Fixed `process` module for `MINGW*`.

### Changed

- Moved `Email::fold_text_plain_parts` to `Parts::to_readable`. It take now a `PartsReaderOptions` as parameter:
  - `plain_first`: shows plain texts first, switch to html if empty.
  - `sanitize`: sanitizes or not text bodies (both plain and html).

### Fixed

- Fixed long subject decoding issue.
- Fixed bad mailbox name encoding from UTF7-IMAP.

## [0.3.1] - 2022-10-10

### Changed

- Renamed `EmailSendCmd` into `SendmailConfig`.
- Renamed `EmailSender::Cmd` into `EmailSender::Sendmail`.

### Fixed

- Fixed broken tests

### Removed

- Removed useless dependency `toml` [patch#1].
  
## [0.3.0] - 2022-10-10

### Changed

- Renamed `DEFAULT_DRAFT_FOLDER` to `DEFAULT_DRAFTS_FOLDER` to be more consistant with IMAP folder names.
- Changed licence to `MIT`.
- Renamed feature `internal-sender` to `smtp-sender`.
  
### Fixed

- Fixed folder name case (because IMAP folders are case sensitive).

## [0.2.1] - 2022-09-29

### Changed

- Removed notmuch from the default features.

## [0.2.0] - 2022-09-28

### Changed

- Unwrapped folders and envelopes from struct:

  ```rust
  // Before
  pub struct Envelopes {
	  pub envelopes: Vec<Envelope>,
  }
  
  // After
  pub struct Envelopes(pub Vec<Envelope>);
  ```

- Renamed `TplOverride::sig` to `TplOverride::signature`.
- Upgraded Nix deps.

### Fixed

- Fixed imap backend pagination overflow.

## [0.1.0] - 2022-09-22

First official version of the Himalaya's library. The source code mostly comes from the [CLI](https://github.com/soywod/himalaya) repository.

[keyring]: https://crates.io/crates/keyring
[lettre]: https://github.com/lettre/lettre
[mail-parser]: https://github.com/stalwartlabs/mail-parser
[mail-send]: https://github.com/stalwartlabs/mail-send
[maildir]: https://github.com/staktrace/maildir
[maildirpp]: https://crates.io/crates/maildirpp
[secret service]: https://specifications.freedesktop.org/secret-service/latest/

[patch#1]: https://lists.sr.ht/~soywod/himalaya-lib/patches/35686
[patch#2]: https://lists.sr.ht/~soywod/pimalaya/patches/39136
[patch#3]: https://lists.sr.ht/~soywod/pimalaya/patches/39154
[patch#4]: https://lists.sr.ht/~soywod/pimalaya/patches/39189
[patch#5]: https://lists.sr.ht/~soywod/pimalaya/patches/39215
[patch#6]: https://lists.sr.ht/~soywod/pimalaya/patches/39215
[patch#7]: https://lists.sr.ht/~soywod/pimalaya/patches/39261

[#6]: https://todo.sr.ht/~soywod/pimalaya/6
[#9]: https://todo.sr.ht/~soywod/pimalaya/9
[#20]: https://todo.sr.ht/~soywod/pimalaya/20
[#36]: https://todo.sr.ht/~soywod/pimalaya/36
[#37]: https://todo.sr.ht/~soywod/pimalaya/37
[#38]: https://todo.sr.ht/~soywod/pimalaya/38
[#44]: https://todo.sr.ht/~soywod/pimalaya/44
[#45]: https://todo.sr.ht/~soywod/pimalaya/45
[#46]: https://todo.sr.ht/~soywod/pimalaya/46
[#50]: https://todo.sr.ht/~soywod/pimalaya/50
[#52]: https://todo.sr.ht/~soywod/pimalaya/52
[#57]: https://todo.sr.ht/~soywod/pimalaya/57
[#61]: https://todo.sr.ht/~soywod/pimalaya/61
[#70]: https://todo.sr.ht/~soywod/pimalaya/70
[#76]: https://todo.sr.ht/~soywod/pimalaya/76
[#80]: https://todo.sr.ht/~soywod/pimalaya/80
[#92]: https://todo.sr.ht/~soywod/pimalaya/92
[#94]: https://todo.sr.ht/~soywod/pimalaya/94
[#95]: https://todo.sr.ht/~soywod/pimalaya/95
[#97]: https://todo.sr.ht/~soywod/pimalaya/97
[#103]: https://todo.sr.ht/~soywod/pimalaya/103
[#110]: https://todo.sr.ht/~soywod/pimalaya/110
[#152]: https://todo.sr.ht/~soywod/pimalaya/152
[#158]: https://todo.sr.ht/~soywod/pimalaya/158
[#169]: https://todo.sr.ht/~soywod/pimalaya/169
[#172]: https://todo.sr.ht/~soywod/pimalaya/172
[#174]: https://todo.sr.ht/~soywod/pimalaya/174
[#179]: https://todo.sr.ht/~soywod/pimalaya/179
[#187]: https://todo.sr.ht/~soywod/pimalaya/187
[#195]: https://todo.sr.ht/~soywod/pimalaya/195

[himalaya#487]: https://github.com/pimalaya/himalaya/issues/487
[himalaya#535]: https://github.com/pimalaya/himalaya/issues/535
[himalaya#536]: https://github.com/pimalaya/himalaya/issues/536
