# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3] - 2023-07-01

### Added

- Added `CmdOutput::read_out()` function that reads command output as string. If the exit code is different than `0`, reads the error output instead.
- Added `CmdOutput::read_out_lossy()` function, same as `read_out()` but lossy.

### Changed

- Made the code async using the tokio async runtime.
- Renamed `CmdOutput::stdout` by `out`.
- Renamed `CmdOutput::stderr` by `err`.

## [0.0.2] - 2023-05-19

### Added

- Added missing implementations `Deref` and `From<String>`.

## [0.0.1] - 2023-05-18

### Added

- Imported process code from `pimalaya-email`.
