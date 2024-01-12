# 💼 Pimalaya [![Matrix](https://img.shields.io/matrix/pimalaya:matrix.org?color=success&label=chat)](https://matrix.to/#/#pimalaya:matrix.org)

[Pimalaya](https://pimalaya.org/) is an ambitious project that aims to improve open-source tools related to Personal Information Management ([PIM](https://en.wikipedia.org/wiki/Personal_information_manager)) which includes emails, contacts, calendars, tasks and more.

This repository tries to accomplish the first mission of Pimalaya, which is providing Rust libraries dedicated to the PIM domain.

| Library     | PIM domain | Description                                                 |
|-------------|------------|-------------------------------------------------------------|
| [email-lib] | Email      | List mailboxes, search envelopes, read/write/send messages… |
| [mml-lib]   | Email      | Read/write messages using the MIME Meta Language            |
| [time-lib]  | Time       | Manage time using customizable client/server timers         |

[email-lib]: https://git.sr.ht/~soywod/pimalaya/tree/master/item/email/README.md 
[mml-lib]: https://git.sr.ht/~soywod/pimalaya/tree/master/item/mml/README.md 
[time-lib]: https://git.sr.ht/~soywod/pimalaya/tree/master/item/time/README.md 

## Development

The development environment is managed by [Nix](https://nixos.org/download.html). Running `nix-shell` will spawn a shell with everything you need to get started with the lib: `cargo`, `cargo-watch`, `rust-bin`, `rust-analyzer`…

```sh
# start a nix shell
$ nix-shell

# build all libs
$ cargo build
```

## Contributing

A **bug tracker** is available at <https://todo.sr.ht/~soywod/pimalaya>.

A **mailing list** is available at <https://lists.sr.ht/~soywod/pimalaya>.

If you want to **report a bug**, please send an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a patch at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht). The simplest way to send a patch is to use [git send-email](https://git-scm.com/docs/git-send-email), follow [this guide](https://git-send-email.io/) to configure git properly.

If you just want to **discuss** about the project, feel free to join the [Matrix](https://matrix.org/) workspace [#pimalaya](https://matrix.to/#/#pimalaya:matrix.org) or contact me directly [@soywod](https://matrix.to/#/@soywod:matrix.org). You can also use the mailing list [[send an email](mailto:~soywod/pimalaya@lists.sr.ht)|[subscribe](mailto:~soywod/pimalaya+subscribe@lists.sr.ht)|[unsubscribe](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht)].

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the [NLnet foundation](https://nlnet.nl/project/Himalaya/index.html) and the [European Commission](https://www.ngi.eu/) that helped the project to receive financial support from:

- [NGI Assure](https://nlnet.nl/assure/) in 2022
- [NGI Zero Entrust](https://nlnet.nl/entrust/) in 2023

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
