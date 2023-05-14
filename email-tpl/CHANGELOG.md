# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed `Message-ID` header not set by default [#49].
- Fixed empty text parts issues [#32].

## [0.1.0] - 2023-02-07

### Added

- Implemented the template parser based on the [Emacs MML] module (only `<#multipart>` and `<#part>`).
- Implemented the template compiler that builds MIME Messages using the message builder of the [lettre] crate.
- Implemented the compiler builder to customize PGP encrypt and sign shell commands.
- Added a template builder.
- Added option `remove_text_plain_parts_signature`.

[Emacs MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/MML-Definition.html
[lettre]: https://github.com/lettre/lettre

[#32]: https://todo.sr.ht/~soywod/pimalaya/32
[#49]: https://todo.sr.ht/~soywod/pimalaya/49
