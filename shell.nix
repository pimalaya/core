{ nixpkgs ? <nixpkgs>
, system ? builtins.currentSystem
, pkgs ? import nixpkgs { inherit system; }
, fenix ? import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { }
, pimalaya ? import (fetchTarball "https://github.com/pimalaya/nix/archive/master.tar.gz")
, extraBuildInputs ? ""
}:

pimalaya.mkShell {
  inherit nixpkgs system pkgs fenix extraBuildInputs;

  rustToolchainFile = ./rust-toolchain.toml;
  buildInputs = with pkgs; [
    openssl.dev
    gnupg
    gpgme
    msmtp
    notmuch
  ];
}
