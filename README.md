# 💼 Pimalaya [![Matrix](https://img.shields.io/matrix/pimalaya:matrix.org?color=success&label=chat)](https://matrix.to/#/#pimalaya:matrix.org)

https://pimalaya.org/

**Pimalaya** is an ambitious project that aims to improve open-source tools in order to better manage our personal information (as known as [PIM]), which includes emails, events, calendars, contacts and more.

**The first objective** of the project is to provide [Rust libraries](https://git.sr.ht/~soywod/pimalaya) containing all this [PIM] logic. They serve as basement for all sort of top-level applications: CLI, TUI, GUI, plugins, servers etc.

<table border="1">
  <thead>
    <tr>
      <th>Library</th>
      <th>
        Pim domain
      </th>
      <th>Links</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>pimalaya-email</td>
      <td>Email management</td>
      <td>
        <a href="https://git.sr.ht/~soywod/pimalaya/tree/master/item/email/README.md">git</a>,
        <a href="https://docs.rs/pimalaya-email/latest/pimalaya_email/">api</a>,
        <a href="https://crates.io/crates/pimalaya-email">crate</a>
      </td>
    </tr>
    <tr>
      <td>pimalaya-email-tpl</td>
      <td>Email management</td>
      <td>
        <a href="https://git.sr.ht/~soywod/pimalaya/tree/master/item/email-tpl/README.md">git</a>,
        <a href="https://docs.rs/pimalaya-email-tpl/latest/pimalaya_email_tpl/">api</a>,
        <a href="https://crates.io/crates/pimalaya-email-tpl">crate</a>
      </td>
    </tr>
    <tr>
      <td>pimalaya-time</td>
      <td>
        Time management
      </td>
      <td>
        <a href="https://git.sr.ht/~soywod/pimalaya/tree/master/item/time/README.md">git</a>,
        <a href="https://docs.rs/pimalaya-time/latest/pimalaya_time/">api</a>,
        <a href="https://crates.io/crates/pimalaya-time">crate</a>
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
      <td>email, email-tpl</td>
      <td>
        CLI (<a href="https://github.com/soywod/himalaya">git</a>, <a href="https://pimalaya.org/himalaya/">doc</a>),
        <a href="https://git.sr.ht/~soywod/himalaya-vim">Vim plugin</a>,
        <a href="https://github.com/dantecatalfamo/himalaya-emacs">Emacs plugin</a>
      </td>
    </tr>
    <tr>
      <td>Comodoro</td>
      <td>time</td>
      <td>
        CLI (<a href="https://github.com/soywod/comodoro">git</a>, <a href="https://pimalaya.org/comodoro/">doc</a>)
      </td>
    </tr>
  </tbody>
</table>

*Disclaimer: the project is under active development, do not use in production before the v1.0.0.*

## Development

The development environment is managed by [Nix](https://nixos.org/download.html). Running `nix-shell` will spawn a shell with everything you need to get started with the lib: `cargo`, `cargo-watch`, `rust-bin`, `rust-analyzer`, `notmuch`…

```sh
# Start a Nix shell
$ nix-shell

# then build all the libs
$ cargo build
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

Special thanks to the [nlnet](https://nlnet.nl/project/Himalaya/index.html) foundation that helped Pimalaya to receive financial support from the [NGI Assure](https://www.ngi.eu/ngi-projects/ngi-assure/) program of the European Commission in September, 2022.

## Sponsoring

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)

[PIM]: https://en.wikipedia.org/wiki/Personal_information_manager
