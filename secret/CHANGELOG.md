# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2024-10-27

### Added

- Added `vendored` cargo feature. The feature is forwarded to `keyring-rs` in order to pack OpenSSL lib with this crate.

### Changed

- Renamed cargo features:
  - `keyring-tokio` has been splitted into `keyring`, `tokio` and `rustls` features
  - `keyring-tokio-openssl` has been splitted into `keyring`, `tokio` and `openssl` features
  - `keyring-async-io` has been splitted into `keyring`, `async-std` and `rustls` features
  - `keyring-async-io-openssl` has been splitted into `keyring`, `async-std` and `openssl` features
- Replaced `log` by `tracing`.
- Renamed `Secret::Undefined` with `Secret::Empty`.
- Renamed `Secret::is_undefined` with `Secret::is_empty`.
- Renamed `Secret::set_only_keyring` with `Secret::set_if_keyring`.
- Renamed `Secret::delete_only_keyring` with `Secret::delete_if_keyring`.
- Renamed `Secret::KeyringEntry` with `Secret::Keyring`.
- Changed `Secret::replace_undefined_to_keyring` with `Secret::replace_if_empty`. This function became more generic. It takes now a new `Secret` as argument instead of a keyring entry.

## [0.4.6] - 2024-08-16

### Fixed

- Prevented `process-lib` and `keyring-lib` to be automatically built when using `derive` feature by using the `?` syntax.

## [0.4.5] - 2024-06-03

### Changed

- Bumped `keyring@0.4.3`.

## [0.4.4] - 2024-04-08

### Changed

- Bumped `keyring@0.4.2`.

## [0.4.3] - 2024-04-06

### Changed

- Bumped `keyring@0.4.1`.
- Bumped `process@0.4.2`.

## [0.4.2] - 2024-04-06

### Changed

- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level to match the previous API.

## [0.4.1] - 2024-03-14

### Changed

- Bumped `process-lib@v0.4.1`.

## [0.4.0] - 2024-03-14

### Added

- Added cargo feature `derive` to enable/disable (de)serialization of the `Secret` structure using `serde`.
- Added cargo feature `command` to enable/disable `Secret::Command` variant.
- Added cargo features `keyring-tokio`, `keyring-tokio-openssl`, `keyring-async-io` and `keyring-async-io-openssl` to enable/disable `Secret::Keyring` variant.
- Added function `Secret::set`.
- Added function `Secret::delete`.

### Changed

- Renamed `Secret::Cmd` to `Secret::Command` for clarity.
- Renamed `Secret::set_keyring_entry_if_undefined` to `Secret::set_only_keyring`.
- Renamed `Secret::delete_keyring_entry_secret` to `Secret::delete_only_keyring`.

## [0.3.3] - 2024-01-12

### Changed

- Bumped `process-lib@0.3.1`.

## [0.3.2] - 2023-12-31

### Changed

- Bumped `keyring-lib@0.3.2`.

## [0.3.1] - 2023-12-31

### Changed

- Bumped `keyring-lib@0.3.1`.

## [0.3.0] - 2023-12-11

### Changed

- Made `Secret` serializable and deserializable using `serde`.
- Made `set_keyring_entry_secret` async.
- Made `delete_keyring_entry_secret` async.

## [0.2.0] - 2023-12-10

### Added

- Exposed publicly `keyring` and `process` at the crate root level, to prevent types version mismatches.

### Changed

- Bumped `keyring-lib@=0.2.0`.
- Bumped `process-lib@=0.2.0`.

## [0.1.0] - 2023-08-27

- Renamed project `secret-lib` in order to make it generic.

## [0.0.5] - 2023-07-09

### Changed

- Bumped `pimalaya_keyring@0.0.5`.

## [0.0.4] - 2023-07-03

### Changed

- Bumped `pimalaya_process@0.0.5`.

## [0.0.3] - 2023-07-02

### Changed

- Made the code async due to `pimalaya_process@0.0.4`.

## [0.0.2] - 2023-06-06

### Added

- Added logs and comments.
- Added `Secret::find`.

### Changed

- Renamed `Secret::Keyring` by `KeyringEntry`.
- Added `Secret::Undefined` variant, which is now the default variant.
- Renamed `Secret::new_keyring` by `new_keyring_entry`.
- Renamed `Secret::is_undefined_entry` by `is_undefined`.
- Renamed `Secret::replace_undefined_entry_with` by `set_keyring_entry_if_undefined`.
- Renamed `Secret::set` by `set_keyring_entry_secret`.
- Renamed `Secret::delete` by `delete_keyring_entry_secret`.

### Removed

- Removed `Error::is_get_secret_error`, use `Secret::find` instead.

## [0.0.1] - 2023-05-19

### Added

- Imported process code from `pimalaya-email`.

[1.0.0]: https://crates.io/crates/secret-lib/1.0.0
[0.4.6]: https://crates.io/crates/secret-lib/0.4.6
[0.4.5]: https://crates.io/crates/secret-lib/0.4.5
[0.4.4]: https://crates.io/crates/secret-lib/0.4.4
[0.4.3]: https://crates.io/crates/secret-lib/0.4.3
[0.4.2]: https://crates.io/crates/secret-lib/0.4.2
[0.4.1]: https://crates.io/crates/secret-lib/0.4.1
[0.4.0]: https://crates.io/crates/secret-lib/0.4.0
[0.3.3]: https://crates.io/crates/secret-lib/0.3.3
[0.3.2]: https://crates.io/crates/secret-lib/0.3.2
[0.3.1]: https://crates.io/crates/secret-lib/0.3.1
[0.3.0]: https://crates.io/crates/secret-lib/0.3.0
[0.2.0]: https://crates.io/crates/secret-lib/0.2.0
[0.1.0]: https://crates.io/crates/secret-lib/0.1.0
[0.0.5]: https://crates.io/crates/pimalaya-secret/0.0.5
[0.0.4]: https://crates.io/crates/pimalaya-secret/0.0.4
[0.0.3]: https://crates.io/crates/pimalaya-secret/0.0.3
[0.0.2]: https://crates.io/crates/pimalaya-secret/0.0.2
[0.0.1]: https://crates.io/crates/pimalaya-secret/0.0.1
