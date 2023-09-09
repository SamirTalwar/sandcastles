{
  description = "sandcastles";

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

      runtimeDependencies = pkgs.lib.optionals pkgs.stdenv.isDarwin [
        pkgs.libiconv
      ];
      buildArguments = {
        src = craneLib.cleanCargoSource (craneLib.path ./.);
        buildInputs = runtimeDependencies;
      };
    in
    {
      packages.sandcastles-deps = craneLib.buildDepsOnly buildArguments;

      packages.sandcastles = craneLib.buildPackage (buildArguments // {
        cargoArtifacts = self.packages.${system}.sandcastles-deps;
        doCheck = false; # checks are complicated; we do them outside Nix
      });

      packages.default = self.packages.${system}.sandcastles;

      devShells.default = pkgs.mkShell {
        nativeBuildInputs = [
          # build
          pkgs.cargo
          pkgs.cargo-edit
          pkgs.cargo-insta
          pkgs.cargo-machete
          pkgs.cargo-nextest
          pkgs.clippy
          pkgs.rust-analyzer
          pkgs.rustPlatform.rustcSrc
          pkgs.rustc
          pkgs.rustfmt

          # testing
          pkgs.bash
          pkgs.coreutils
          pkgs.nodejs
          pkgs.nushell
        ];

        buildInputs = runtimeDependencies;
      };

      formatter = pkgs.nixpkgs-fmt;
    });
}
