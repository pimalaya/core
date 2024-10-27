# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-10-27

### Added

- Added `tokio` and `async-std` cargo features (they are both mutually exclusive).
- Added `rustls` and `native-tls` cargo features (they are both mutually exclusive).
- Added `vendored` cargo feature.
- Added `Client` main structure with `new` and `send` functions.
