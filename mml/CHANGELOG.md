# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
