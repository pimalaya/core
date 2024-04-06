# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added a root level transparent `Error` and `Result`, wrapping errors from other modules.

### Changed

- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level of `v2_0` module to match the previous API.

## [0.1.0] - 2023-08-27

- Renamed project `oauth-lib` in order to make it generic.

## [0.0.4] - 2023-07-20

### Changed

- Made code async using `tokio`.

## [0.0.3] - 2023-06-06

### Added

- Added the Refresh Access Token flow builder.
- Added the Client builder.

### Changed

- Changed `AuthorizationCodeFlow::wait_for_redirection`: it takes now a reference to a `BasicClient`.
- Moved `Error` to their respective module.
- Moved `AuthorizationCodeFlow::get_client` to its own module `client`.

## [0.0.2] - 2023-05-19

### Added

- Added more examples and documentation.

## [0.0.1] - 2023-05-03

### Added

- Imported process code from `pimalaya-email`.
