# 📫 Himalaya lib

Rust library for email management.

```rust
let account_config = AccountConfig {
    email: "test@localhost".into(),
    display_name: Some("Test".into()),
    email_sender: EmailSender::Internal(SmtpConfig {
        host: "localhost".into(),
        port: 587,
        starttls: Some(true),
        login: "login".into(),
        passwd_cmd: "echo password".into(),
        ..Default::default()
    }),
    ..Default::default()
};

let imap_config = ImapConfig {
    host: "localhost".into(),
    port: 993,
    starttls: Some(true),
    login: "login".into(),
    passwd_cmd: "echo password".into(),
    ..Default::default()
};
let backend_config = BackendConfig::Imap(&imap_config);

let mut backend = BackendBuilder::new().build(&account_config, &backend_config).unwrap();
backend.list_envelopes("INBOX", 10, 0).unwrap();
backend.move_email("INBOX", "Archives", "21").unwrap();
backend.delete_email("INBOX", "42").unwrap();

let mut sender = SenderBuilder::build(&account_config).unwrap();
let email = Email::from("To: test2@localhost\r\nSubject: Hello\r\n\r\nContent");
sender.send(&account_config, &email).unwrap();
```

*The project is under active development. Do not use in production
before the `v1.0.0`.*

## Introduction

The role of this library is to extract and expose an <abbr
title="application programming interface">API</abbr> for managing
emails. This way, you can build clients that match the best your
workflow without reiventing the wheel. Here the list of available
clients built by the community:

* [<abbr title="command-line
  interface">CLI</abbr>](https://github.com/soywod/himalaya)
* [Vim plugin](https://git.sr.ht/~soywod/himalaya-vim)
* [Emacs plugin](https://github.com/dantecatalfamo/himalaya-emacs)
* <abbr title="graphical user interface">GUI</abbr> (coming soon)
* <abbr title="text-based user interfaces">TUI</abbr>
* Web server
* …

## Features

- [IMAP](https://en.wikipedia.org/wiki/Internet_Message_Access_Protocol),
  [Maildir](https://en.wikipedia.org/wiki/Maildir) and
  [Notmuch](https://notmuchmail.org/) backends
- [SMTP](https://en.wikipedia.org/wiki/Simple_Mail_Transfer_Protocol)
  and [Sendmail](https://en.wikipedia.org/wiki/Sendmail) senders
- List, add and delete folders (mailboxes)
- List and search envelopes
- Get, add, copy, move and delete emails
- Add, set and delete flags
- Multi-accounting
- Folder aliases
- <abbr title="Pretty Good Privacy">PGP</abbr> end-to-end encryption
- <abbr title="Internet Message Access Protocol">IMAP</abbr> IDLE mode
  for real-time notifications
- …

## Development

The development environment is managed by
[Nix](https://nixos.org/download.html). Running `nix-shell` will spawn
a shell with everything you need to get started with the lib: `cargo`,
`cargo-watch`, `rust-bin`, `rust-analyzer`, `notmuch`…

```shell-session
# Starts a Nix shell
$ nix-shell

# then builds the lib
$ cargo build
```

## Testing

Before running the test suite you need to spawn an IMAP server. Here
an example with [`docker`](https://www.docker.com/) and
[`greenmail`](https://github.com/greenmail-mail-test/greenmail):

```shell-session
$ docker run -it --rm \
  -p 3025:3025 -p 3110:3110 -p 3143:3143 -p 3465:3465 -p 3993:3993 -p 3995:3995 \
  -e GREENMAIL_OPTS='-Dgreenmail.setup.test.all -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled -Dgreenmail.verbose' \
  greenmail/standalone:1.6.2
  
$ cargo test
```

## Contributing

If you find a **bug**, please send an email at
[~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you have a **question**, please send an email at
[~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a
patch at
[~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht)
using [git send-email](https://git-scm.com/docs/git-send-email) (see
[this guide](https://git-send-email.io/) on how to configure it).

If you want to **subscribe** to the mailing list, please send an email
at
[~soywod/pimalaya+subscribe@lists.sr.ht](mailto:~soywod/pimalaya+subscribe@lists.sr.ht).

If you want to **unsubscribe** to the mailing list, please send an
email at
[~soywod/pimalaya+unsubscribe@lists.sr.ht](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht).

If you want to **discuss** about the project, feel free to join the
[Matrix](https://matrix.org/) workspace
[#pimalaya](https://matrix.to/#/#pimalaya:matrix.org) or contact me
directly [@soywod](https://matrix.to/#/@soywod:matrix.org).

## Credits

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/project/Himalaya/index.html)

Special thanks to the
[nlnet](https://nlnet.nl/project/Himalaya/index.html) foundation that
helped Himalaya to receive financial support from the [NGI
Assure](https://www.ngi.eu/ngi-projects/ngi-assure/) program of the
European Commission in September, 2022.

## Sponsoring

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors&style=flat-square)](https://github.com/sponsors/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff&style=flat-square)](https://www.paypal.com/paypalme/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff&style=flat-square)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000&style=flat-square)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222&style=flat-square)](https://liberapay.com/soywod)
