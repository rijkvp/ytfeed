{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };
  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        inherit (nixpkgs) lib;
        pkgs = nixpkgs.legacyPackages.${system};
      in
      with pkgs;
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "ytfeed";
          inherit ((lib.importTOML (self + "/Cargo.toml")).package) version;
          src = self;
          cargoLock.lockFile = self + "/Cargo.lock";
        };
        devShells.default = mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo clippy cargo-outdated ];
        };
      }
    );
}

