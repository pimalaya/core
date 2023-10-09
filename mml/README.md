# 📫 MIME Meta Language

Rust implementation of the Emacs MIME message Meta Language, as known as [MML](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html).

This library exposes a MML to MIME message compiler and a MIME to MML message interpreter. See the [API documentation](https://docs.rs/mml-lib/latest/mml/) for more information.

For example:

```eml
From: alice@localhost
To: bob@localhost
Subject: MML examples

This is a plain text part.

<#part type=text/html>
<h1>This is a HTML part.</h1>
<#/part>

<#part filename=./examples/attachment.png description="This is an attachment."><#/part>
```

compiles to:

```eml
MIME-Version: 1.0
From: <alice@localhost>
To: <bob@localhost>
Subject: MML examples
Message-ID: <17886a741feef4a2.f9706245cd3a3f97.3b41d60ef9e2fbfb@soywod>
Date: Tue, 26 Sep 2023 09:58:26 +0000
Content-Type: multipart/mixed; 
	boundary="17886a741fef2cb2_97a7dbff4c84bbac_3b41d60ef9e2fbfb"


--17886a741fef2cb2_97a7dbff4c84bbac_3b41d60ef9e2fbfb
Content-Type: text/plain; charset="utf-8"
Content-Transfer-Encoding: 7bit

This is a plain text part.


--17886a741fef2cb2_97a7dbff4c84bbac_3b41d60ef9e2fbfb
Content-Type: text/html; charset="utf-8"
Content-Transfer-Encoding: 7bit

<h1>This is a HTML part.</h1>

Content-Type: application/octet-stream
Content-Disposition: attachment; filename="attachment.png"
Content-Transfer-Encoding: base64

iVBORw0KGgo…

--17886a741fef2cb2_97a7dbff4c84bbac_3b41d60ef9e2fbfb--
```

## Definition

From the [Emacs documentation](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/MML-Definition.html):

> Creating a MIME message is boring and non-trivial. Therefore, a library called mml has been defined that parses a language called MML (MIME Meta Language) and generates MIME messages.
>
> The MML language is very simple. It looks a bit like an SGML application, but it’s not.
> 
> The main concept of MML is the part. Each part can be of a different type or use a different charset. The way to delineate a part is with a ‘<#part ...>’ tag. Multipart parts can be introduced with the ‘<#multipart ...>’ tag. Parts are ended by the ‘<#/part>’ or ‘<#/multipart>’ tags. Parts started with the ‘<#part ...>’ tags are also closed by the next open tag.
> 
> […]
> 
> Each tag can contain zero or more parameters on the form ‘parameter=value’. The values may be enclosed in quotation marks, but that’s not necessary unless the value contains white space. So ‘filename=/home/user/#hello$^yes’ is perfectly valid.

## Features

- MML to MIME messages compilation using [`MmlCompilerBuilder`](https://docs.rs/mml-lib/latest/mml/message/compiler/struct.MmlCompilerBuilder.html)  (cargo feature `compiler` required, enabled by default)
- MIME to MML messages interpretation using the [`MimeInterpreterBuilder`](https://docs.rs/mml-lib/latest/mml/message/interpreter/struct.MimeInterpreterBuilder.html) (cargo feature `interpreter` required, activated by default)
- Multiple parts support `<#multipart>…<#/multipart>`
- Inline part support `<#part text=mime/type>…<#/part>`
- Attachment support `<#part disposition=attachment filename=/path/to/attachment.ext><#/part>`
- Comment support `<#!part>This will not be compiled<#!/part>`
- PGP support:
  - Shell commands (cargo feature `pgp-commands` required)
  - GPG bindings (cargo feature `pgp-gpg` and [`gpgme`](https://gnupg.org/software/gpgme/index.html) lib required)
  - Native Rust implementation (cargo feature `pgp-native` required)

## Examples

See [`./examples`](https://git.sr.ht/~soywod/pimalaya/tree/master/item/mml/examples):

```sh
cargo run --example
```

## Development

The development environment is managed by [Nix](https://nixos.org/download.html). Running `nix-shell` will spawn a shell with everything you need to get started with the lib: `cargo`, `cargo-watch`, `rust-bin`, `rust-analyzer`…

```sh
# Start a Nix shell
$ nix-shell

# then build the lib
$ cargo build -p mml-lib
```

## Contributing

If you want to **report a bug** that [does not exist yet](https://todo.sr.ht/~soywod/pimalaya), please send an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a patch at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht) using [git send-email](https://git-scm.com/docs/git-send-email). Follow [this guide](https://git-send-email.io/) to configure git properly.

If you just want to **discuss** about the project, feel free to join the [Matrix](https://matrix.org/) workspace [#pimalaya.general](https://matrix.to/#/#pimalaya.general:matrix.org) or contact me directly [@soywod](https://matrix.to/#/@soywod:matrix.org). You can also use the mailing list [[send an email](mailto:~soywod/pimalaya@lists.sr.ht)|[subscribe](mailto:~soywod/pimalaya+subscribe@lists.sr.ht)|[unsubscribe](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht)].

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the [NLnet foundation](https://nlnet.nl/project/Himalaya/index.html) and the [European Commission](https://www.ngi.eu/) that helped the project to receive financial support from:

- [NGI Assure](https://nlnet.nl/assure/) in 2022
- [NGI Zero Untrust](https://nlnet.nl/entrust/) in 2023

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
