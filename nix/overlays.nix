{ inputs }:

{
  default = (final: prev:
    let
      builderPkgs = import inputs.nixpkgs {
        overlays = [ (import inputs.rust-overlay) ];
        inherit final;
      };
      naersk = builderPkgs.callPackage naersk {
        rustc = builderPkgs.rust-bin.nightly.latest.default;
        cargo = builderPkgs.rust-bin.nightly.latest.default;
      };
      packages = import ./default.nix { pkgs = builderPkgs; inherit naersk; };
    in {
      v5d = packages.v5d;
      v5ctl = packages.v5ctl;
    });
}
