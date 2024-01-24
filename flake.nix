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
      }
    );
}
