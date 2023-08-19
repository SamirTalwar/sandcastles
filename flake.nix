{
  description = "Boof";

  inputs = {
    flake-utils.url = github:numtide/flake-utils;
    nixpkgs.url = github:NixOS/nixpkgs/master;
    crane = {
      url = github:ipetkov/crane;
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , flake-utils
    , nixpkgs
    , crane
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };
      craneLib = crane.lib.${system};
    in
    {
      packages.boof = craneLib.buildPackage {
        src = craneLib.cleanCargoSource (craneLib.path ./.);
      };

      packages.default = self.packages.${system}.boof;

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = [
          # build
          pkgs.cargo
          pkgs.cargo-edit
          pkgs.cargo-insta
          pkgs.cargo-machete
          pkgs.clippy
          pkgs.rust-analyzer
          pkgs.rustPlatform.rustcSrc
          pkgs.rustc
          pkgs.rustfmt

          # testing
          pkgs.nodejs
          pkgs.nushell
        ];

        buildInputs = [
          pkgs.libiconv
        ];
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
