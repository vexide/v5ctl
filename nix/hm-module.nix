self:
{ config, lib, pkgs, ... }:

with lib;
let cfg = config.services.v5d;
in {
  options = {
    services.v5d = {
      enable = mkEnableOption "v5d";
      connectionType = mkOption {
        type = types.str;
        default = "auto";
        example = "bluetooth";
        description =
          "The method to use for connecting to the V5 Brain. Can be 'auto', 'serial', or 'bluetooth'";
      };
      package = mkOption {
        type = types.package;
        default = self.packages.${pkgs.stdenv.targetPlatform.system}.v5d;
        description = "The package to use for the V5 Brain daemon.";
      };
    };
  };

  config = (mkIf cfg.enable {
    systemd.user.services.v5d = {
      Unit = { Description = "VEX V5 Brain daemon."; };
      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/v5d -c ${cfg.connectionType}";
        Restart = "on-failure";
      };
    };
  });
}
