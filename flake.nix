{
  description = "A Rust implementation of the V5 Serial Protocol";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    naersk.url = "github:nix-community/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, systems, rust-overlay, naersk, ... }:
    let eachSystem = nixpkgs.lib.genAttrs (import systems);
    in {
      devShells = eachSystem (system:
        let
          pkgs = import nixpkgs {
            overlays = [ (import rust-overlay) ];
            inherit system;
          };
        in {
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
        });
      packages = eachSystem (system:
        let
          pkgs = import nixpkgs {
            overlays = [ (import rust-overlay) ];
            inherit system;
          };
          naersk' = pkgs.callPackage naersk {
            rustc = pkgs.rust-bin.nightly.latest.default;
            cargo = pkgs.rust-bin.nightly.latest.default;
          };
        in (import ./nix {
          inherit pkgs;
          naersk = naersk';
        }).packages);
      homeManagerModules = eachSystem (system:
        let
          pkgs = import nixpkgs {
            overlays = [ (import rust-overlay) ];
            inherit system;
          };
          naersk' = pkgs.callPackage naersk {
            rustc = pkgs.rust-bin.nightly.latest.default;
            cargo = pkgs.rust-bin.nightly.latest.default;
          };
          module = (import ./nix {
            inherit pkgs;
            naersk = naersk';
          }).hm-module;
        in rec {
          v5d = module;
          default = v5d;
        });
    };
}
