{ pkgs, naersk, ... }:

naersk.buildPackage {
  src = ../.;
  pname = "v5d";
  version = "0.1.0";

  nativeBuildInputs = with pkgs; [ pkg-config dbus udev ];
}
