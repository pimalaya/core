# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3] - 2024-04-06

### Changed

- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level to match the previous API.

## [0.0.2] - 2023-06-03

### Fixed

- Fixed doc issues.

## [0.0.1] - 2023-06-02

### Added

- Forked crate from [maildir], adjust code to remove mutable borrows and `mailparse`.

[maildir]: https://github.com/staktrace/maildir
