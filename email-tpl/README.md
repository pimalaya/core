# 📫 pimalaya-email-tpl

Rust library for interpreting and compiling MIME Messages based on the [Emacs MIME Meta Language](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html):

> Creating a MIME message is boring and non-trivial. Therefore, a library called mml has been defined that parses a language called MML (MIME Meta Language) and generates MIME messages.

```eml
From: alice@localhost
To: bob@localhost
Subject: MML simple

<#multipart type=alternative>
This is a plain text part.
<#part type=text/enriched>
<center>This is a centered enriched part</center>
<#/multipart>
```

compiles to:

```eml
Subject: MML simple
To: bob@localhost
From: alice@localhost
MIME-Version: 1.0
Date: Tue, 29 Nov 2022 13:07:01 +0000
Content-Type: multipart/alternative;
 boundary="4CV1Cnp7mXkDyvb55i77DcNSkKzB8HJzaIT84qZe"

--4CV1Cnp7mXkDyvb55i77DcNSkKzB8HJzaIT84qZe
Content-Type: text/plain; charset=utf-8
Content-Transfer-Encoding: 7bit

This is a plain text part.
--4CV1Cnp7mXkDyvb55i77DcNSkKzB8HJzaIT84qZe
Content-Type: text/enriched
Content-Transfer-Encoding: 7bit

<center>This is a centered enriched part</center>
--4CV1Cnp7mXkDyvb55i77DcNSkKzB8HJzaIT84qZe--
```

## Definition

From the [documentation](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/MML-Definition.html):

> The MML language is very simple. It looks a bit like an SGML application, but it’s not.
> 
> The main concept of MML is the part. Each part can be of a different type or use a different charset. The way to delineate a part is with a ‘<#part ...>’ tag. Multipart parts can be introduced with the ‘<#multipart ...>’ tag. Parts are ended by the ‘<#/part>’ or ‘<#/multipart>’ tags. Parts started with the ‘<#part ...>’ tags are also closed by the next open tag.
> 
> […]
> 
> Each tag can contain zero or more parameters on the form ‘parameter=value’. The values may be enclosed in quotation marks, but that’s not necessary unless the value contains white space. So ‘filename=/home/user/#hello$^yes’ is perfectly valid.

## Features

- Interpret raw emails as template using the `TplInterpreter` builder
- Compile templates as raw emails using the template compiler builder
- Multiple parts support `<#multipart>…<#/multipart>`
- Inline part support `<#part text=mime/type>…<#/part>`
- Attachment support `<#part filename=/path/to/attachment.ext>`
- PGP support using system commands (encrypt, decrypt, sign, verify)

## Examples

See [`./examples`](https://git.sr.ht/~soywod/pimalaya/tree/master/item/email-tpl/examples):

```sh
cargo run --example
```

## Development

The development environment is managed by [Nix](https://nixos.org/download.html). Running `nix-shell` will spawn a shell with everything you need to get started with the lib: `cargo`, `cargo-watch`, `rust-bin`, `rust-analyzer`, `notmuch`…

```sh
# Start a Nix shell
$ nix-shell

# then build the lib
$ cargo build -p pimalaya-email-tpl
```

## Contributing

If you find a **bug** that [does not exist yet](https://todo.sr.ht/~soywod/pimalaya), please send an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you have a **question**, please send an email at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a patch at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht) using [git send-email](https://git-scm.com/docs/git-send-email) (see [this guide](https://git-send-email.io/) on how to configure it).

If you want to **subscribe** to the mailing list, please send an email at [~soywod/pimalaya+subscribe@lists.sr.ht](mailto:~soywod/pimalaya+subscribe@lists.sr.ht).

If you want to **unsubscribe** to the mailing list, please send an email at [~soywod/pimalaya+unsubscribe@lists.sr.ht](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht).

If you want to **discuss** about the project, feel free to join the [Matrix](https://matrix.org/) workspace [#pimalaya](https://matrix.to/#/#pimalaya:matrix.org) or contact me directly [@soywod](https://matrix.to/#/@soywod:matrix.org).

## Credits

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the [nlnet](https://nlnet.nl/project/Himalaya/index.html) foundation that helped Himalaya to receive financial support from the [NGI Assure](https://www.ngi.eu/ngi-projects/ngi-assure/) program of the European Commission in September, 2022.

## Sponsoring

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors&style=flat-square)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff&style=flat-square)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff&style=flat-square)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000&style=flat-square)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222&style=flat-square)](https://liberapay.com/soywod)
