{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
  };
  outputs =
    {
      nixpkgs,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
        commonArgs = {
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              (craneLib.fileset.rust ./.)
              (craneLib.fileset.cargoTomlAndLock ./.)
            ];
          };
          doCheck = false;
          strictDeps = true;
        };
        crate = craneLib.buildPackage (
          commonArgs
          // {
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          }
        );
      in
      {
        packages.default = crate;
        apps.default = flake-utils.lib.mkApp {
          drv = crate;
        };
        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            cargo-edit
          ];
        };
      }
    );
}
