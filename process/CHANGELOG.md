# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2024-10-27

### Added

- Added `tokio` and `async-std` cargo features (they are mutually exclusive).

### Changed

- Replaced `log` by `tracing`.
- Renamed `SingleCommand` to `Command`.

### Removed

- Removed `Command` enum.

## [0.4.2] - 2024-04-06

### Changed

- Moved `Error` and `Result` into a dedicated `error` module. They are still re-exported at the root level to match the previous API.

## [0.4.1] - 2024-03-14

### Added

- Added cargo feature `derive` to enable/disable (de)serialization of structures using `serde`.

## [0.4.0] - 2024-02-02

### Changed

- Renamed `Cmd` to `Command`.
- Renamed `Cmd::SingleCmd` to `SingleCommand`.
- Renamed `SingleCmd` to `SingleCommand` and turned it into a unit struct.
- Renamed `CmdOutput` to `CommandOutput` and turned it into a unit struct.
- Renamed `Error::InvalidExitStatusCodeNonZeroError` to `GetExitStatusCodeNonZeroError`.

### Removed

- Removed unused `Error` variants `WaitForExitStatusCodeError`, `WriteStdinError`, `ReadStdoutError`, `ReadStderrError` and `GetOutputError`.

## [0.3.1] - 2024-01-12

### Fixed

- Fixed `Cmd` and `Pipeline` serialization issues.

## [0.3.0] - 2023-12-11

### Changed

- Made `Cmd`, `SingleCmd` and `Pipeline` serializable and deserializable using `serde`.

## [0.2.0] - 2023-12-10

### Added

- Added `SingleCmd::with_output_piped` to control whenever stdout and stderr should be piped or not.

## [0.1.0] - 2023-08-27

### Changed

- Renamed project `process-lib` in order to make it generic.

## [0.0.5] - 2023-07-02

### Added

- Implemented `Into<Vec<u8>>` for `CmdOutput`.

### Changed

- Moved `CmdOuput::try_into_string` to `TryFrom<String>`.

## [0.0.4] - 2023-07-02

### Changed

- Changed the way exit code is handled: it now returns an error `InvalidExitStatusCodeNonZeroError` when the exit code is different than `0`, and the exit code is not accessible anymore from `CmdOutput`.
- Changed `CmdOutput` shape, it is now a simple unit struct holding the output as `Vec<u8>`.
- Renamed `CmdOutput::read_out()` by `try_into_string`.
- Renamed `CmdOutput::read_out_lossy()` by `to_string_lossy`.

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

[1.0.0]: https://crates.io/crates/process-lib/1.0.0
[0.4.2]: https://crates.io/crates/process-lib/0.4.2
[0.4.1]: https://crates.io/crates/process-lib/0.4.1
[0.4.0]: https://crates.io/crates/process-lib/0.4.0
[0.3.1]: https://crates.io/crates/process-lib/0.3.1
[0.3.0]: https://crates.io/crates/process-lib/0.3.0
[0.2.0]: https://crates.io/crates/process-lib/0.2.0
[0.1.0]: https://crates.io/crates/process-lib/0.1.0
[0.0.5]: https://crates.io/crates/pimalaya-process/0.0.5
[0.0.4]: https://crates.io/crates/pimalaya-process/0.0.4
[0.0.3]: https://crates.io/crates/pimalaya-process/0.0.3
[0.0.2]: https://crates.io/crates/pimalaya-process/0.0.2
[0.0.1]: https://crates.io/crates/pimalaya-process/0.0.1
