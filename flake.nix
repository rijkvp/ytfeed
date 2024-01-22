{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.lib.${system};
        crate = with pkgs; craneLib.buildPackage {
          src = craneLib.cleanCargoSource (craneLib.path ./.);
        };
      in
      {
        checks = { inherit crate; };
        packages = {
          default = crate;
        };
        apps.default = flake-utils.lib.mkApp {
          drv = crate;
        };
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            cargo-outdated
          ];
        };

        # NixOS module to run it as systemd service
        nixosModules.ytfeed = { config, lib, pkgs, ... }:
          let
            cfg = config.services.ytfeed;
          in
          with lib;
          {
            options.services.ytfeed = {
              enable = mkEnableOption "ytfeed";
              user = mkOption {
                default = "ytfeed";
                type = types.str;
                description = "User to run ytfeed as";
              };
              group = mkOption {
                default = "ytfeed";
                type = types.str;
                description = "Group to run ytfeed as";
              };
            };
            users.users.ytfeed = optionalAttrs (cfg.user == "ytfeed") {
              isSystemUser = true;
              group = cfg.group;
            };
            users.groups.ytfeed = optionalAttrs (cfg.group == "ytfeed") { };

            config = mkIf cfg.enable {
              systemd.services.ytfeed = {
                description = "ytfeed";
                wantedBy = [ "multi-user.target" ];
                after = [ "network.target" ];
                serviceConfig = {
                  ExecStart = "${crate}/bin/ytfeed";
                  User = cfg.user;
                  Group = cfg.group;
                  Environment = {
                    "RUST_LOG" = "info";
                    "RUST_BACKTRACE" = "1";
                  };
                };
              };
            };
          };
      }
    );
}

