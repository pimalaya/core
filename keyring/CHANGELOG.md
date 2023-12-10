# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
