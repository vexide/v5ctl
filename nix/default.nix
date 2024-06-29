{ naersk, pkgs, ... }: {
  v5d = pkgs.callPackage ./v5d.nix { inherit naersk; };
  v5ctl = pkgs.callPackage ./v5ctl.nix { inherit naersk; };
}
