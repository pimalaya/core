# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Put `serde` support behind cargo feature `derive`, disabled by default.

## [0.2.1] - 2024-02-03

### Changed

- Improved logs to better know what is going on during server and timer lifecycles.

## [0.2.0] - 2024-02-02

### Changed

- Made the code asynchronous, using `tokio`.
- Made server and timer handlers asynchronous as well.
- Moved `ServerStream::read` and `ServerStream::write` to `request::RequestReader` and `response::ResponseWriter`.
- Changed `ServerStream` signature to `ServerStream: RequestReader + ResponseWriter`
- Moved `ClientStream::read` and `ServerStream::write` to `response::ResponseReader` and `request::RequestWriter`.
- Changed `ClientStream` signature to `ClientStream: ResponseReader + RequestWriter`

## [0.1.1] - 2023-10-09

### Added

- Improved inline documentation, examples and tests.

### Changed

- Exposed all modules to public API.

## [0.1.0] - 2023-08-27

### Changed

- Renamed project `time-lib` in order to make it generic.

## [0.0.2] - 2023-06-24

### Added

- Added ability to customize the number of cycles the timer should do via the enum `TimerLoop`. `TimerLoop::Infinite` loops indefinitely (same behaviour as before, and it is the default), `TimerLoop::Fixed(usize)` loops n times before stopping by itself.
- Added `Timer::elapsed()` that returns the amount of time the timer was running, in seconds.

### Change

- Added `TimerConfig::cycles_count` (which expects a `TimerLoop`).
- Replaced the timer iterator by `Timer::update()`, which updates the inner state of the current timer based on `Timer::elapsed()`.
- Added `Timer::cycles_count` (which expects a `TimerLoop`).
- Added `Timer::started_at` (which expects a `Option<Instant>`) that holds the instant moment where the timer has been started for the last time.
- Added `Timer::elapsed` (which expects a `usize`) that holds the elapsed time, in seconds, from the first start till the last start.

### Fixed

- Fixed timer accuracy. Over time, the timer was slower and slower, loosing accuracy. Now the timer is not an iterator anymore and uses `std::time::Instant` instead to make sure the elapsed time and cycle are correct [#91].

## [0.0.1] - 2023-05-18

### Added

- Added `ServerBuilder` struct.
- Added server handler and timer handlers.
- Added code documentation with scheme.

### Changed

- Changed the name and the aim of the project. The timer is not Pomodoro-specific anymore, it became generic (which allows you to turn it into a Pomodoro timer, or whatever).

## [0.0.0] - 2023-03-12

[#91]: https://todo.sr.ht/~soywod/pimalaya/91
