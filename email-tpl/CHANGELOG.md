# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added 3 new cargo features:
  - `cmds-pgp`: enables the commands PGP backend (enabled by default, same behaviour as before)
  - `gpg`: enables the GPG backend (requires the `gpgme` lib on the system)
  - `native-pgp`: enables the native PGP backend

### Changed

- Changed the way PGP operations are done. Encryption/decryption/signing/verifying can be done using shell commands (like before), using GPG via `gpgme`, or using a native implementation of the OpenPGP standard via `rPGP`. They are grouped into a new enum called `Pgp`.
- Prefixed compiler and interpreter builder functions with `with_` to match more the Builder Pattern.
- Replaced `TplCompiler::pgp_encrypt_cmd` and `TplCompiler::pgp_verify_cmd` by `with_pgp` which takes a `Into<Pgp>`.
- Replaced `TplInterpreter::pgp_decrypt_cmd` and `TplInterpreter::pgp_sign_cmd` by `with_pgp` which takes a `Into<Pgp>`.

### Fixed

- Fixed multipart template compiling to empty body [#99].

## [0.3.1] - 2023-07-18

### Fixed

- Fixed default PGP verify command that was using invalid option `--recipient`.

## [0.3.0] - 2023-07-03

### Changed

- Made the code async due to `pimalaya_process@0.0.5`.

## [0.2.3] - 2023-06-15

### Fixed

- Fixed missing `<` and `>` when displaying `Message-ID` and `In-Reply-To` headers.

## [0.2.2] - 2023-06-15

### Changed

- Added space between list of addresses (after the comma).

## [0.2.1] - 2023-06-10

### Fixed

- Fixed top level imports.

## [0.2.0] - 2023-06-03

### Added

- Added parsing template from raw message support. Parsing is done via the `TplInterpreter` builder, and functions `TplInterpreter::interpret_*` return the parsed template.

### Changed

- Replaced [lettre] by [mail-builder] and [mail-parser].
- Use crate [nanohtml2text] instead of manual html to plain transform using ammonia, html-escape and regex.
- Moved MML stuff in its own `mml` module, to be as close as what provides the Emacs MML module. The `tpl` module contains stuff related to template. A template is just an email composed of headers and one unique plain text part. This plain text part can be written in MML.
- Compiler options are now attached to the `Tpl` structure.

## [0.1.1] - 2023-05-19

### Changed

- Replaced process management with `pimalaya-process`.

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
[mail-builder]: https://github.com/stalwartlabs/mail-builder
[mail-parser]: https://github.com/stalwartlabs/mail-parser
[nanohtml2text]: https://crates.io/crates/nanohtml2text

[#32]: https://todo.sr.ht/~soywod/pimalaya/32
[#49]: https://todo.sr.ht/~soywod/pimalaya/49
[#99]: https://todo.sr.ht/~soywod/pimalaya/99
