{
  lib,
  config,
  pkgs,
  ...
}:

let
  cfg = config.wg-mesh;
in
{
  options.wg-mesh = {
    enable = lib.mkEnableOption "wg-mesh";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.wg-mesh;
      defaultText = lib.literalExpression "pkgs.wg-mesh";
      description = "The wg-mesh package to use.";
    };

    domain = lib.mkOption {
      type = lib.types.str;
      description = ''
        DNS name under which the mesh (list of peers) is published, passed
        as the argument to wg-mesh.
      '';
    };

    interface = lib.mkOption {
      type = lib.types.str;
      default = "wg0";
      description = "Name of the wireguard network interface used for the mesh.";
    };
  };

  # this module only manages the wg-mesh service itself; setting up the
  # interface (netdev/network/firewall/private key) is left to the consumer,
  # since that is deployment-specific.
  config = lib.mkIf cfg.enable {
    systemd.services.wg-mesh = {
      description = "wg-mesh";
      # it's possible that wg0 is not available when this unit is started, wg-mesh then crashes and systemd should restart the unit
      wants = [
        "sys-subsystem-net-devices-${cfg.interface}.device"
      ];
      after = [
        "sys-subsystem-net-devices-${cfg.interface}.device"
      ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/wg-mesh ${cfg.domain}";
        Restart = "on-failure";
        RestartSec = 10;
      };
    };
  };
}
