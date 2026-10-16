# NixOS module for zapret-rust.
#
# The package is set by the flake (see flake.nix). Without the flake, set
# services.zapret-rust.package yourself.
{ config, lib, pkgs, ... }:

let
  cfg = config.services.zapret-rust;

  boolStr = b: if b then "true" else "false";

  # Same keys the app reads and writes in conf.env.
  configFile = pkgs.writeText "zapret-rust-conf.env" ''
    engine=${cfg.engine}
    interface=${cfg.interface}
    strategy=${cfg.strategy}
    gamefiltertcp=${boolStr cfg.gamefilter.tcp}
    gamefilterudp=${boolStr cfg.gamefilter.udp}
    backend=${cfg.backend}
    active_discord_fake=quic_initial_steamcommunity_com.bin
    active_gamefilter_fake=quic_initial_4pda_to.bin
    dpi_desync_ttl=${lib.optionalString (cfg.dpiDesyncTtl != null) (toString cfg.dpiDesyncTtl)}
    editor=
    backup_lists=true
  '';
in
{
  options.services.zapret-rust = {
    enable = lib.mkEnableOption "zapret-rust DPI bypass service";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The zapret-rust package to run.";
    };

    engine = lib.mkOption {
      type = lib.types.enum [ "zapret" "zapret2" ];
      default = "zapret";
      description = "Engine generation: zapret (nfqws) or zapret2 (nfqws2).";
    };

    strategy = lib.mkOption {
      type = lib.types.str;
      example = "discord.bat";
      description = "Strategy file to run. Must match a strategy the engine provides.";
    };

    interface = lib.mkOption {
      type = lib.types.str;
      default = "any";
      description = "Network interface to bind firewall rules to, or \"any\".";
    };

    backend = lib.mkOption {
      type = lib.types.enum [ "nftables" "iptables" ];
      default = "nftables";
      description = "Firewall backend used for NFQUEUE rules.";
    };

    gamefilter = {
      tcp = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable the game filter for TCP (zapret engine only).";
      };
      udp = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable the game filter for UDP (zapret engine only).";
      };
    };

    dpiDesyncTtl = lib.mkOption {
      type = lib.types.nullOr lib.types.ints.u8;
      default = null;
      description = "TTL override for desync packets. null uses the strategy default.";
    };

    cacheDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/zapret-rust";
      description = "Directory for downloaded engine binaries and strategies.";
    };

    nixLd = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Enable nix-ld so dynamically linked engine binaries downloaded at runtime
        can run on NixOS. Not needed if the downloaded binaries are static.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.strategy != "";
        message = "services.zapret-rust.strategy must be set when zapret-rust is enabled.";
      }
    ];

    environment.systemPackages = [ cfg.package ];
    environment.etc."zapret-rust/conf.env".source = configFile;

    systemd.tmpfiles.rules = [
      "d ${cfg.cacheDir} 0755 root root -"
    ];

    systemd.services.zapret-rust = {
      description = "zapret-rust DPI bypass";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-pre.target" ];
      restartTriggers = [ configFile ];

      serviceConfig = {
        # The app runs in the foreground and removes its firewall rules on SIGTERM.
        ExecStart = "${cfg.package}/bin/zapret-rust --config /etc/zapret-rust/conf.env --cache-dir ${cfg.cacheDir}";
        Restart = "on-failure";
        RestartSec = 5;
      };
    };

    programs.nix-ld = lib.mkIf cfg.nixLd {
      enable = true;
      libraries = with pkgs; [
        libnetfilter_queue
        libnfnetlink
        libmnl
        zlib
        libcap
      ];
    };
  };
}
