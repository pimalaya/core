# 💼 Pimalaya [![Matrix](https://img.shields.io/matrix/pimalaya.general:matrix.org?color=success&label=chat)](https://matrix.to/#/#pimalaya.general:matrix.org)

<https://pimalaya.org/>

**Pimalaya** is an ambitious project that aims to improve open-source tools related to Personal Information Management ([PIM]) which includes emails, contacts, calendars, tasks and more.

**The first objective** of the project is to provide [Rust libraries](https://git.sr.ht/~soywod/pimalaya) containing all this [PIM] logic. They serve as basement for all sort of top-level applications: CLI, TUI, GUI, plugins, servers etc.

<table border="1">
  <thead>
    <tr>
      <th>Library</th>
      <th>
        Description
      </th>
      <th>Links</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>email-lib</td>
      <td>Email management</td>
      <td>
        <a href="https://git.sr.ht/~soywod/pimalaya/tree/master/item/email/README.md">git</a>,
        <a href="https://docs.rs/email-lib/latest/email/">api</a>,
        <a href="https://crates.io/crates/email-lib">crate</a>
      </td>
    </tr>
    <tr>
      <td>mml-lib</td>
      <td>MIME Meta Language</td>
      <td>
        <a href="https://git.sr.ht/~soywod/pimalaya/tree/master/item/mml/README.md">git</a>,
        <a href="https://docs.rs/mml-lib/latest/mml/">api</a>,
        <a href="https://crates.io/crates/mml-lib">crate</a>
      </td>
    </tr>
    <tr>
      <td>time-lib</td>
      <td>Time management</td>
      <td>
        <a href="https://git.sr.ht/~soywod/pimalaya/tree/master/item/time/README.md">git</a>,
        <a href="https://docs.rs/time-lib/latest/time/">api</a>,
        <a href="https://crates.io/crates/time-lib">crate</a>
      </td>
    </tr>
  </tbody>
</table>

**The second objective** is to provide quality house-made applications built at the top of those libraries.

<table border="1">
  <thead>
    <tr>
      <th>Project</th>
      <th>Libraries used</th>
      <th>Links</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Himalaya</td>
      <td>email-lib, mml-lib</td>
      <td>
        CLI (<a href="https://github.com/soywod/himalaya">git</a>, <a href="https://pimalaya.org/himalaya/">doc</a>),
        <a href="https://git.sr.ht/~soywod/himalaya-vim">Vim</a>,
        <a href="https://github.com/dantecatalfamo/himalaya-emacs">Emacs</a>,
        <a href="https://www.raycast.com/jns/himalaya">Raycast</a>
      </td>
    </tr>
    <tr>
      <td>MML</td>
      <td>mml-lib</td>
      <td>
        CLI (<a href="https://github.com/soywod/mml">git</a>, <a href="https://pimalaya.org/mml/">doc</a>),
        <a href="https://git.sr.ht/~soywod/mml-vim">Vim</a>
      </td>
    </tr>
    <tr>
      <td>Comodoro</td>
      <td>time-lib</td>
      <td>
        CLI (<a href="https://github.com/soywod/comodoro">git</a>, <a href="https://pimalaya.org/comodoro/">doc</a>),
        <a href="https://www.raycast.com/jns/comodoro">Raycast</a>
      </td>
    </tr>
  </tbody>
</table>

*Disclaimer: the project is under active development, do not use in production before the v1.0.0.*

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

[PIM]: https://en.wikipedia.org/wiki/Personal_information_manager
