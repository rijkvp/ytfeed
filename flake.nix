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
          buildInputs = lib.optionals stdenv.isDarwin [
            pkgs.libiconv
          ];
        };
        dockerImage = pkgs.dockerTools.buildImage {
          name = crate.pname;
          tag = "latest";
          copyToRoot = [ crate ];
          config = {
            Cmd = [ "${crate}/bin/${crate.pname}" ];
          };
        };
      in
      {
        checks = { inherit crate; };
        packages = {
          inherit crate dockerImage;
          default = crate;
        };
        apps.default = flake-utils.lib.mkApp {
          drv = crate;
        };
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.cargo-outdated
          ];
        };
      }
    );
}

