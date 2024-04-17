{
  description = "Rust library to manage your personal information (PIM).";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust-toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "+syqAd2kX8KVa8/U2gz3blIQTTsYYt3U63xBWaGOSc8=";
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [
            # Nix env
            nil
            nixpkgs-fmt

            # Rust env
            rust-toolchain
            cargo-watch

            # Email env
            openssl.dev
            gnupg
            gpgme
            msmtp
            notmuch
          ];
        };

        # TODO: find a way to cargo test
      });
}
