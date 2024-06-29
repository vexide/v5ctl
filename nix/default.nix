{ naersk, pkgs, ... }: rec {
  packages = {
    v5d = pkgs.callPackage ./v5d.nix { inherit naersk; };
    v5ctl = pkgs.callPackage ./v5ctl.nix { inherit naersk; };
  };

  hm-module = import ./hm-module.nix {
    v5d = packages.v5d;
    lib = pkgs.lib;
    config = pkgs.config;
  };
}
