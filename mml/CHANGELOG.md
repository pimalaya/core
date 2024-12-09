# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.1] - 2024-12-09

### Added

- Added `MimeBodyInterpreter::show_parts` to customize whenever MML markup should be shown or not for basic parts.

### Fixed

- Fixed missing `compiler` feature guard on error variants.

## [1.1.0] - 2024-10-27

### Added

- Added `tokio` and `async-std` cargo features (they are both mutually exclusive).
- Added `rustls` and `native-tls` cargo features (they are both mutually exclusive)
- Added `secret-keyring` and `secret-command` cargo features (they are forwarded to the `secret-lib` crate)
- Added `Pgp::None` variant with associated `Error::PgpMissingConfigurationError` error.

### Changed

- Renamed `Pgp::Cmds` to `Pgp::Commands`.
- Renamed `Gpg` to `PgpGpg`.
- Renamed `CmdsPgp` to `PgpCommands`.
- Renamed `NativePgp` to `PgpNative`.

## [1.0.14] - 2024-08-16

### Fixed

- Prevented `process-lib` and `secret-lib` to be automatically built when using `derive` feature by using the `?` syntax.

## [1.0.13] - 2024-06-03

### Changed

- Bumped `secret@0.4.5`.

## [1.0.12] - 2024-04-09

### Removed

- Removed `chumsky@1.0.0-alpha.7`'s `spill-stack` feature due to cross-compilation issues.

## [1.0.11] - 2024-04-08

### Changed

- Bumped `secret@0.4.4`.

## [1.0.10] - 2024-04-07

### Fixed

- Bumped `chumsky@1.0.0-alpha.7` that fixes wrong diagnosis labels.

## [1.0.9] - 2024-04-06

### Changed

- Removed `.trim()` when interpreting MML string.
- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level to match the previous API.

### Fixed

- Fixed `gpg` backend that was not using armored content.

## [1.0.8] - 2024-03-14

### Changed

- Bumped dependencies.

### Added

- Added cargo feature `derive` to enable/disable (de)serialization of structs using `serde`.

## [1.0.7] - 2024-01-12

### Changed

- Bumped `process-lib@0.3.1`.
- Bumped `secret-lib@0.3.3`.

## [1.0.6] - 2023-12-31

### Changed

- Bumped `keyring-lib@0.3.2`.
- Bumped `secret-lib@0.3.2`.

## [1.0.5] - 2023-12-31

### Changed

- Bumped `keyring-lib@0.3.1`.
- Bumped `secret-lib@0.3.1`.

## [1.0.4] - 2023-12-19

### Changed

- Bumped `chumsky@v1.0.0-alpha.6` due to release error.

## [1.0.3] - 2023-12-11

### Changed

- Made `NativePgp`, `CmdsPgp` and `Gpg` serializable and deserializable using `serde`.

## [1.0.2] - 2023-12-10

### Added

- Added `set_pgp`, `set_some_pgp` and `with_some_pgp` utils for compiler and interpreter.

### Changed

- Replaced `warn!` with `debug!`.
- Internal `Pgp` for compiler and interpreter are now optional `Option<Ppg>`, it should not have any impact on the API.

## [1.0.1] - 2023-10-09

### Changed

- Bumped `chumsky@v1.0.0-alpha.5`.
- Bumped `mail-parser@v0.9`.

### Fixed

- Fixed `encoding` property not set properly.

## [1.0.0] - 2023-09-27

### Changed

- Renamed `CompileMmlResult` to `MmlCompileResult`.
- Improved inline docs.

## [0.5.1] - 2023-09-26

### Added

- Added `doc_auto_cfg` feature to have feature tags on `docs.rs`.

## [0.5.0] - 2023-09-25

### Added

- Added `MmlCompilerBuilder` and `MimeInterpreterBuilder`. Their `build()` function respectively return a `MmlCompiler` and a `MimeInterpreter`.

### Changed

- Changed the return type of `MmlCompiler::compile`. It now returns a `CompileMmlResult`, where you can get a MIME message using one of the following function: `into_msg_builder`, `into_vec` and `into_string`.
- Renamed `MimeInterpreter::interpret_*` by `from_*`.
- Renamed `Part::MultiPart` to `Multi`.
- Renamed `Part::SinglePart` to `Singe`.
- Renamed `Part::TextPlainPart` to `PlainText`.
- Merged `Part::Attachment` with `Part::SinglePart`. The `filename` prop is no longer mandatory. Instead, if set, the content of the given file overrides the inline body of the part.

## [0.4.0] - 2023-09-20

### Added

- Added `FilterHeaders::Exclude` variant.
- Added `MimeBodyInterpreter::default_save_attachments_dir` function.

### Changed

- Renamed `ShowHeadersStrategy` to `FilterHeaders` to match `FilterParts`.
- Renamed `ShowHeadersStrategy::Only` to `FilterHeaders::Include`.
- Improved MML parser new line labels.

## [0.3.2] - 2023-09-17

### Fixed

- Fixed tests and examples.

## [0.3.1] - 2023-09-17

### Changed

- Improved inline documentation for docs.rs.

## [0.3.0] - 2023-09-17

### Added

- Added the `description` prop parser, which correspond to the `Content-Description` header.
- Added full of examples in `./examples`.

### Changed

- Renamed module `pgp::cmds` to `pgp::commands` to match the cargo feature.

## [0.2.3] - 2023-08-29

### Changed

- Renamed `pimalaya-shellexpand` to `shellexpand-utils`.

## [0.2.2] - 2023-08-29

### Changed

- Replaced `shellexpand` by `pimalaya-shellexpand`.

## [0.2.1] - 2023-08-27

## Fixed

- Fixed internal dependencies paths.

## [0.2.0] - 2023-08-27

## Changed

- Renamed feature `pgp-cmds` to `pgp-commands`, and removed it from default features. PGP needs now to be manually activated.

## [0.1.2] - 2023-08-27

### Removed

- Removed `chumsky`'s `spill-stack` feature on windows due to incompatibility issue.

## [0.1.1] - 2023-08-27

### Fixed

- Fixed missing angles when compiling MML containing one of those headers: Message-ID, References, In-Reply-To, Return-Path, Content-ID, Resent-Message-ID.

## [0.1.0] - 2023-08-23

### Added

- Imported code from `pimalaya-email-tpl`.

[#487]: https://github.com/pimalaya/himalaya/issues/487
