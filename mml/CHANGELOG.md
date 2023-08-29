# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
