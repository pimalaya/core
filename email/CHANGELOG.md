# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added OAuth 2.0 support for IMAP [#9].
- Added [Secret Service] support via the [keyring] crate [#6].
- Added `ImapAuthConfig` struct that contains config related to OAuth
  2.0. It also contains a `configure` method to set up client secret
  and store access token from redirect URL using [Secret Service].

### Changed

- Replaced `ImapConfig::passwd_cmd` with `ImapConfig::auth` which
  takes now 4 variants:
  - `ImapAuthConfig::None` (default)
  - `ImapAuthConfig::RawPasswd` for raw clear password authentication
    (not recommanded)
  - `ImapAuthConfig::PasswdCmd` equivalent of the previous
    `ImapConfig::passwd_cmd`
  - `ImapAuthConfig::OAuth2` for OAuth 2.0 authentication

### Removed

- Removed `rustls-native-certs` cargo feature, it is now included by
  default within the `rustls-tls` cargo feature.

## [0.7.0] - 2023-05-01

### Added

- Initiated `.gitattributes` file [patch#4].
- Added new account option `sync_folders_strategy` which allows to
  choose a folders synchronization strategy:
  - `Strategy::All`: synchronize all existing folders
  - `Strategy::Include`: synchronize only the given folders
  - `Strategy::Exclude`: synchronizes all folders except the given
    ones
- Added warning message when `process::run` exit code is not `0`
  [patch#6].
- Added `vendored` feature (linked to the `native-tls/vendored` one).

### Changed

- Changed the way `Flag::Custom` is used: in order to have a more
  unified API across backends, the custom variant is only used when
  receiving data (not anymore when parsing data from
  backends). Therefore custom flags are not synchronized anymore
  (because custom flags are not supported by the Maildir backend).
- Returns an error if `BackendSyncBuilder::sync` cannot acquire the
  lock in order to avoid processes to block each other infinitely
  [patch#7].
- Made `rustls` the default feature over `native-tls` to improve
  compatibility among operating systems.

### Fixed

- Fixed date parsing using the [mail-parser] crate [#44].
- Fixed Cc addresses when replying all [#46].
- Clarified header/value trace logs [patch#2].
- Fixed default `imap-notify-cmd` placeholders not being replaced
  [patch#3].
- Fixed IMAP session pool errors [#50].
- Fixed wrong recipient when from = sender [#52].
- Fixed invalid `ProcessEnvelopesPatch` length [#57].
- Fixed the process.rs' `pipe` function so it returns exit code
  correctly [patch#5].
- Fixed notmuch folders management [#45].

### Removed

- Removed `serde::Serialize` trait from structures and `serde` deps.
- Removed variant `Recent` from flag.
- Removed `Flag::to_symbols_string`: the responsibility shifted client
  side.

## [0.6.0] - 2023-02-14

### Added

- Added ability to synchronize specific folders only [#37].
- Added `Backend::expunge` function that definitely removes emails
  with the `Deleted` flag.
- Added `Backend::mark_emails_as_deleted` function with a default
  implementation that adds the `Deleted` flag.

### Changed

- Changed the way emails are deleted. `Backend::delete_emails` now
  moves the email to the `Trash` folder (or to the corresponding alias
  from the config file). If the target folder is the `Trash` folder,
  it will instead add the `Deleted` flag. Emails are removed with the
  `Backend::expunge` function.

### Fixed

- Fixed `ImapBackend::list_envelopes` pagination.
- Fixed synchronization issues for emails without `Message-ID` header
  by using the `Date` header instead.
- Fixed maildir backend perfs issues by enabling the `mmap` feature of
  the `maildir` crate.
  
### Removed

- Removed the `maildir-backend` cargo feature, it is now included by
  default.

## [0.5.1] - 2023-02-08

### Fixed

- Fixed `notmuch` backend compilation error on rustc `v1.67+`.

## [0.5.0] - 2023-02-07

### Added

- Made backend functions accept a vector of id instead of a single id
  [#20].
- Added function `Backend::purge_folder` that removes all emails
  inside a folder.
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
- Added emails synchronization feature. Backends that implement the
  `ThreadSafeBackend` trait inherit the `sync` function that
  synchronizes all folders and emails with a local `Maildir` instance.
- Added `Backend::sync` function and link `ThreadSafeBackend::sync` to
  it for the IMAP and the Maildir backends.
- Added the ability to URL encode Maildir folders (in order to fix
  path collisions, for eg `[Gmail]/Sent`). Also added a
  `MaildirBackendBuilder` to facilitate the usage of the
  `url_encoded_folders` option.
- Added a process lock for `ThreadSafeBackend::sync`, this way only
  one synchronization can be performed at a time (for a same account).

### Fixed

- Used native IMAP commands `copy` and `mv`.
- Fixed maildir date envelope parsing.
- Fixed inline attachments not collected.

### Changed

- Improved `Backend` method names. Also replaced the `self mut` by a
  `RefCell`.
- Simplified the `Email` struct: there is no custom implementation
  with custom fields. Now, the `Email` struct is just a wrapper around
  `mailparse::ParsedMail`.
- Improved `Flag` structures.
- Changed `Backend` trait functions due to [#20]:
  - `list_envelope` => `list_envelopes`
  - `search_envelope` => `search_envelopes`
  - `get_email` => `get_emails`, takes now `ids: Vec<&str>` and
    returns an `Emails` structure instead of an `Email`)
  - `copy_email` => `copy_emails`, takes now `ids: Vec<&str>`.
  - `move_email` => `move_emails`, takes now `ids: Vec<&str>`.
  - `delete_email` => `delete_emails`, takes now `ids: Vec<&str>`.
  - `add_flags` takes now `ids: Vec<&str>` and `flags: &Flags`.
  - `set_flags` takes now `ids: Vec<&str>` and `flags: &Flags`.
  - `remove_flags` takes now `ids: Vec<&str>` and `flags: &Flags`.

### Removed

- The `email::Tpl` structure moved to its [own
  repository](https://git.sr.ht/~soywod/mime-msg-builder).
- Encryption and signed moved with the `email::Tpl` in its own
  repository.

## [0.4.0] - 2022-10-12

### Added

- Added pipe support for `(imap|smtp)-passwd-cmd`.
- Added `imap-ssl` and `smtp-ssl` options to be able to disable
  encryption.
- Implemented sendmail sender.
- Fixed `process` module for `MINGW*`.

### Changed

- Moved `Email::fold_text_plain_parts` to `Parts::to_readable`. It
  take now a `PartsReaderOptions` as parameter:
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

- Renamed `DEFAULT_DRAFT_FOLDER` to `DEFAULT_DRAFTS_FOLDER` to be more
  consistant with IMAP folder names.
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

First official version of the Himalaya's library. The source code
mostly comes from the [CLI](https://github.com/soywod/himalaya)
repository.

[mail-parser]: https://github.com/stalwartlabs/mail-parser
[Secret Service]: https://specifications.freedesktop.org/secret-service/latest/
[keyring]: https://crates.io/crates/keyring

[patch#1]: https://lists.sr.ht/~soywod/himalaya-lib/%3C20220929084520.98165-1-me%40paulrouget.com%3E
[patch#2]: https://lists.sr.ht/~soywod/pimalaya/%3C20230219010155.108114-1-me%40djha.skin%3E
[patch#3]: https://lists.sr.ht/~soywod/pimalaya/%3C20230219125611.1776646-1-piotr%40machura.xyz%3E
[patch#4]: https://lists.sr.ht/~soywod/pimalaya/%3C20230220214559.945-1-me%40djha.skin%3E
[patch#5]: https://lists.sr.ht/~soywod/pimalaya/%3C20230221185145.90120-1-me%40djha.skin%3E
[patch#6]: https://lists.sr.ht/~soywod/pimalaya/%3C20230221185145.90120-1-me%40djha.skin%3E#%3C20230221185145.90120-2-me@djha.skin%3E
[patch#7]: https://lists.sr.ht/~soywod/pimalaya/patches/39261

[#9]: https://todo.sr.ht/~soywod/pimalaya/9
[#20]: https://todo.sr.ht/~soywod/pimalaya/20
[#37]: https://todo.sr.ht/~soywod/pimalaya/37
[#44]: https://todo.sr.ht/~soywod/pimalaya/44
[#45]: https://todo.sr.ht/~soywod/pimalaya/45
[#46]: https://todo.sr.ht/~soywod/pimalaya/46
[#50]: https://todo.sr.ht/~soywod/pimalaya/50
[#52]: https://todo.sr.ht/~soywod/pimalaya/52
[#57]: https://todo.sr.ht/~soywod/pimalaya/57

[0.6.0]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.6.0
[0.5.1]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.5.1
[0.5.0]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.5.0
[0.4.0]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.4.0
[0.3.1]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.3.1
[0.3.0]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.3.0
[0.2.1]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.2.1
[0.2.0]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.2.0
[0.1.0]: https://git.sr.ht/~soywod/himalaya-lib/refs/v0.1.0
