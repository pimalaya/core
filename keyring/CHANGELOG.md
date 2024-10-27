# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.2] - 2024-10-27

### Changed

- Improved inline docs and README.
- Renamed cargo `native` dependency by `keyring-native`.

## [1.0.1] - 2024-10-26

### Fixed

- Fixed cargo features for `docs.rs`.

## [1.0.0] - 2024-10-26

### Added

- Added `vendored` cargo feature. The feature is forwarded to `keyring-rs` in order to pack OpenSSL lib with this crate.

### Changed

- Bumped `keyring-rs@v3.6`.
- Replaced `log` by `tracing`.
- Renamed cargo features:
  - `tokio` has been splitted into `tokio` and `rustls` features
  - `tokio-openssl` has been splitted into `tokio` and `openssl` features
  - `async-io` has been splitted into `async-std` and `rustls` features
  - `async-io-openssl` has been splitted into `async-std` and `openssl` features

### Fixed

- Fixed `async-std` support.

### Removed

- Removed `keyutils` caching as it is now handled by `keyring-rs@v3` internally [#222].

## [0.4.3] - 2024-06-03

### Fixed

- Fixed keyutils wrong target used.

## [0.4.2] - 2024-04-08

### Fixed

- Fixed keyutils used on Windows, preventing the lib to build on this platform.

## [0.4.1] - 2024-04-06

### Changed

- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level to match the previous API.

## [0.4.0] - 2024-03-14

### Added

- Added cache system based on `keyutils` (only on Linux machines).
- Added cargo feature `derive` to enable/disable (de)serialization of `KeyringEntry` using `serde`.

### Changed

- Renamed `Entry` to `KeyringEntry` in order to be more explicit.
- Renamed `KeyringEntry::new` to `KeyringEntry::try_new`, as the native entry is now declared once and stored inside `KeyringEntry`.
- Moved `get_global_service_name` and `set_global_service_name` to the module `service`.

## [0.3.2] - 2023-12-31

### Removed

- Removed unused `secret-service` dependency.

## [0.3.1] - 2023-12-31

### Changed

- Bumped `keyring@2.2.0`.
- Changed `keyring` cargo features to default ones.

## [0.3.0] - 2023-12-11

### Changed

- Made `Entry` serializable and deserializable using `serde`.
- `Entry::{get,find,set,delete}_secret` are now `async`.

## [0.2.0] - 2023-12-10

### Added

- Added `set_global_service_name` function to globally change the service name.
- Added `secret-service` as a dependency, to prevent [build issues](https://github.com/hwchen/keyring-rs/issues/148).

### Changed

- Replaced native `keyring` cargo feature by `linux-no-secret-service` by `linux-secret-service-rt-tokio-crypto-rust`. `linux-no-secret-service` was using `keyutils` under the hood, which stores secrets in memory and was loosing them after reboots. A better version in the future would be to use `keyutils` as a cache.

## [0.1.0] - 2023-08-27

- Renamed project `keyring-lib` in order to make it generic.

## [0.0.5] - 2023-07-09

### Changed

- Pinned keyring version `keyring@2.0.4`.

### Removed

- Disabled `keyring` crate builtin secret service on Linux (provided by the default feature `linux-secret-service`), replaced instead by the default Linux kernel keyutils (provided by the feature `linux-no-secret-service`).

## [0.0.4] - 2023-06-06

### Added

- Added `Entry::get_key`.
- Added `Error::FindSecretError`.
- Implemented `Into<String>` for `Entry`.

### Changed

- Renamed `Entry::get` by `get_secret`.
- Renamed `Entry::find` by `find_secret`.
- Renamed `Entry::set` by `set_secret`.
- Renamed `Entry::delete` by `delete_secret`.
- Changed error returned by `Entry::find_secret` from `Error::GetSecretError` to `FindSecretError`.

## [0.0.3] - 2023-06-06

### Added

- Added `Entry::find`.
- Exposed `keyring::Error` as `KeyringError`.

## [0.0.2] - 2023-06-06

### Added

- Added debug logs.
- Added comments.
- Added one basic example.

## [0.0.1] - 2023-05-18

### Added

- Imported keyring code from `pimalaya-email`.

[1.0.0]: https://crates.io/crates/keyring-lib/1.0.0
[0.4.3]: https://crates.io/crates/keyring-lib/0.4.3
[0.4.2]: https://crates.io/crates/keyring-lib/0.4.2
[0.4.1]: https://crates.io/crates/keyring-lib/0.4.1
[0.4.0]: https://crates.io/crates/keyring-lib/0.4.0
[0.3.2]: https://crates.io/crates/keyring-lib/0.3.2
[0.3.1]: https://crates.io/crates/keyring-lib/0.3.1
[0.3.0]: https://crates.io/crates/keyring-lib/0.3.0
[0.2.0]: https://crates.io/crates/keyring-lib/0.2.0
[0.1.0]: https://crates.io/crates/keyring-lib/0.1.0
[0.0.5]: https://crates.io/crates/pimalaya-keyring/0.0.5
[0.0.4]: https://crates.io/crates/pimalaya-keyring/0.0.4
[0.0.3]: https://crates.io/crates/pimalaya-keyring/0.0.3
[0.0.2]: https://crates.io/crates/pimalaya-keyring/0.0.2
[0.0.1]: https://crates.io/crates/pimalaya-keyring/0.0.1

[#222]: https://github.com/hwchen/keyring-rs/pull/222
