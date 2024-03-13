# 🔐 keyring-lib

Manage credentials using OS-specific keyrings: `Secret Service` on Linux, `Security Framework` on MacOS and `Security Credentials` on Windows.

This library aims to be a High-level API for [keyring](https://crates.io/crates/keyring), a cross-platform library to manage credentials, and can be seen as a convenient wrapper around it:

- Made the lib async using `tokio`.
- Simplified cargo features: `tokio` by default, `tokio-openssl`, `async-io` and `async-io-openssl` available.
- Added the cargo feature `serde` that enables serialization and deserialization of a keyring entry from and to a `String`.
- Changed the way the service name is declared: instead of declaring it everytime you declare a keyring entry, you just need to declare it once at the beginning of you program, using the function `keyring::set_global_service_name`.
- Added new function `find_secret` that returns a `Result<Option<String>>`.
- Enabled logging using the [log](https://crates.io/crates/log) crate.
- Added keyring cache based on the linux [keyutils](https://man7.org/linux/man-pages/man7/keyutils.7.html) keyring (only works on Linux machines).

See the full [API documentation](https://docs.rs/keyring-lib/latest/keyring/) and [some examples](https://git.sr.ht/~soywod/pimalaya/tree/master/item/keyring/tests).

```rust
use keyring::{set_global_service_name, KeyringEntry};

#[tokio::main]
async fn main() {
    // define the global keyring service name once
    set_global_service_name("example");

    // create a keyring entry from a key string
    let entry = KeyringEntry::try_new("key").unwrap();

	// define a secret
    entry.set_secret("secret").await.unwrap();

	// get a secret
	entry.get_secret().await.unwrap();

	// find a secret
	entry.find_secret().await.unwrap();

	// deletea secret
    entry.delete_secret().await.unwrap();
}
```

## Development

The development environment is managed by [Nix](https://nixos.org/download.html). Running `nix-shell` will spawn a shell with everything you need to get started with the lib: `cargo`, `cargo-watch`, `rust-bin`, `rust-analyzer`…

```shell
# Start a Nix shell
$ nix-shell

# then build the lib
$ cargo build -p keyring-lib
```

## Contributing

A **bug tracker** is available on [SourceHut](https://todo.sr.ht/~soywod/pimalaya). <sup>[[send an email](mailto:~soywod/pimalaya@todo.sr.ht)]</sup>

A **mailing list** is available on [SourceHut](https://lists.sr.ht/~soywod/pimalaya). <sup>[[send an email](mailto:~soywod/pimalaya@lists.sr.ht)] [[subscribe](mailto:~soywod/pimalaya+subscribe@lists.sr.ht)] [[unsubscribe](mailto:~soywod/pimalaya+unsubscribe@lists.sr.ht)]</sup>

If you want to **report a bug**, please send an email at [~soywod/pimalaya@todo.sr.ht](mailto:~soywod/pimalaya@todo.sr.ht).

If you want to **propose a feature** or **fix a bug**, please send a patch at [~soywod/pimalaya@lists.sr.ht](mailto:~soywod/pimalaya@lists.sr.ht). The simplest way to send a patch is to use [git send-email](https://git-scm.com/docs/git-send-email), follow [this guide](https://git-send-email.io/) to configure git properly.

If you just want to **discuss** about the project, feel free to join the [Matrix](https://matrix.org/) workspace [#pimalaya](https://matrix.to/#/#pimalaya:matrix.org) or contact me directly [@soywod](https://matrix.to/#/@soywod:matrix.org). You can also use the mailing list.

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
