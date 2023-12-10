# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
