{
  description = "Rust library for managing your personal information (PIM).";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, ... }:
    utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs { inherit system overlays; };
          rust-bin = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              # Nix LSP + formatter
              rnix-lsp
              nixpkgs-fmt

              # Rust env
              openssl.dev
              pkg-config
              rust-bin
              rust-analyzer
              cargo-watch

              # GPG
              gnupg
            ];
          };
        }
      );
}
