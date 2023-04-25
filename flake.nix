{
  description = "Rust library for managing your personal information (PIM).";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-22.11";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, naersk, ... }:
    flake-utils.lib.eachDefaultSystem (system: {
      devShells.default =
        let
          pkgs = import nixpkgs { inherit system; };
          rust-toolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "eMJethw5ZLrJHmoN2/l0bIyQjoTX1NsvalWSscTixpI=";
          };
        in
        pkgs.mkShell {
          buildInputs = with pkgs;
            [
              # Nix env
              rnix-lsp
              nixpkgs-fmt

              # Rust env
              rust-toolchain
            ];
        };
    });
}
