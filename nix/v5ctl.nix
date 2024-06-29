{ pkgs, naersk, ... }:

naersk.buildPackage {
  src = ../.;
  pname = "v5ctl";
  version = "0.1.0";

  nativeBuildInputs = with pkgs; [ pkg-config dbus udev ];
}
