{
  description = "A Rust implementation of the V5 Serial Protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs@{ nixpkgs, flake-utils, rust-overlay, naersk, self, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          overlays = [ (import rust-overlay) ];
          inherit system;
        };
        naersk' = pkgs.callPackage naersk {
          rustc = pkgs.rust-bin.nightly.latest.default;
          cargo = pkgs.rust-bin.nightly.latest.default;
        };
      in {
        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              (rust-bin.nightly.latest.default.override {
                extensions = [ "rust-analyzer" "rust-src" ];
              })
              pkg-config
              dbus
              udev
            ];
          };
        };
        packages = import ./nix {
          inherit pkgs;
          naersk = naersk';
        };
      }) // {
        homeManagerModules = rec {
          v5d = import ./nix self;
          default = v5d;
        };
        overlays.default = import ./nix/overlays.nix { inherit inputs; };
      };
}
