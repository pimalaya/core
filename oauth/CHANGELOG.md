# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2024-12-09

### Changed

- Changed `Client::new` signature: `client_secret` is now optional `Option<impl ToString>`. This way the client secret is only sent to OAuth flows when present.

## [1.0.0] - 2024-10-28

### Added

- Added `tokio` and `async-std` cargo features (they are both mutually exclusive).
- Added `rustls` and `native-tls` cargo features (they are both mutually exclusive).
- Added `vendored` cargo feature.

### Changed

- Replaced `reqwest` by Pimalaya core `http` crate.
- Bumped `oauth2@v5.0.0-rc.1`.
- Changed `Client::new` signature: it requires now 3 additional arguments `redirect_scheme: impl ToString`, `redirect_host: impl ToString` and `redirect_port: impl Into<u16>`.

### Removed

- Removed `Client::with_redirect_host`.
- Removed `Client::with_redirect_port`.

## [0.1.1] - 2024-04-06

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

[1.0.0]: https://crates.io/crates/oauth-lib/1.0.0
[0.1.1]: https://crates.io/crates/oauth-lib/0.1.1
[0.1.0]: https://crates.io/crates/oauth-lib/0.1.0
[0.0.4]: https://crates.io/crates/pimalaya-oauth2/0.0.4
[0.0.3]: https://crates.io/crates/pimalaya-oauth2/0.0.3
[0.0.2]: https://crates.io/crates/pimalaya-oauth2/0.0.2
[0.0.1]: https://crates.io/crates/pimalaya-oauth2/0.0.1
