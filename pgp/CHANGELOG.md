# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2024-10-27

### Added

- Added `tokio` and `async-std` cargo features (they are both mutually exclusive).
- Added `rustls` and `native-tls` cargo features (they are both mutually exclusive).
- Added `vendored` cargo feature.

## [0.3.0] - 2024-10-22

### Changed

- Moved `http.rs` to `http/mod.rs`.
- Moved `hkp` and `wkd` modules inside `http` module.
- Bumped `hyper@1.50` and `hyper-rustls@0.27.3`.

## [0.2.0] - 2024-04-06

### Changed

- Changed hash algorithm from `sha1` to `sha256`.
- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level to match the previous API.
- Exposed publicly `sign::{PublicKeyOrSubkey, SignedSecretKeyOrSubkey}`.

### Fixed

- Fixed wrong secret key taken for signing.

## [0.1.0] - 2023-08-27

### Added

- Initiated repository with `encrypt`, `decrypt`, `sign` and `verify` PGP operations.
- Added [Web Key Directory](https://wiki.gnupg.org/WKD) support.
- Added [Key Server](https://en.wikipedia.org/wiki/Key_server_(cryptographic)) support (HTTP and HKP protocols).
- Added utils to generate a key pair, to read secret/public keys from path and to read signature from bytes.

[1.0.0]: https://crates.io/crates/pgp-lib/1.0.0
[0.3.0]: https://crates.io/crates/pgp-lib/0.3.0
[0.2.0]: https://crates.io/crates/pgp-lib/0.2.0
[0.1.0]: https://crates.io/crates/pgp-lib/0.1.0
