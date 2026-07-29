{
  inputs = {
    # keep-sorted start block=yes
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # keep-sorted end
  };

  outputs =
    inputs@{
      self,
      ...
    }:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.git-hooks-nix.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem =
        {
          config,
          system,
          pkgs,
          ...
        }:
        let
          rustfmt = pkgs.fenix.default.rustfmt;
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.fenix.overlays.default
            ];
            config = { };
          };

          packages = rec {
            default = pkgs.rustPlatform.buildRustPackage {
              name = "gardener";
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;
            };
            gardener = default;
          };

          pre-commit.settings = {
            settings.rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {
              lockFile = ./Cargo.lock;
            };
            hooks = {
              cargo-check.enable = true;
              cargo-sort.enable = true;
              clippy = {
                enable = true;
                settings.allowedLints = [ "clippy::pedantic" ];
                settings.denyWarnings = true;
              };
              rustfmt = {
                enable = true;
                package = rustfmt;
              };

              nixfmt.enable = true;
              statix.enable = true;

              keep-sorted.enable = true;
            };
          };

          devShells = {
            default = pkgs.mkShell {
              inherit (config.pre-commit) shellHook;

              packages =
                with pkgs;
                [
                  cargo
                  rustc
                  rustfmt
                  rust-analyzer
                  rustPackages.clippy
                ]
                ++ config.pre-commit.settings.enabledPackages;

              RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            };
          };

          formatter = inputs.treefmt-nix.lib.mkWrapper pkgs {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.keep-sorted.enable = true;
          };

          checks =
            let
              checkArgs = {
                inherit self pkgs;
              };
            in
            {
              hello-world-server = import ./nixos/tests/foo.nix checkArgs;
            };

        };

      flake.nixosModules.default =
        { ... }:
        {
          imports = [
            ./nixos
          ];

          nixpkgs.overlays =
            let
              overlay =
                final: prev:
                let
                  inherit (prev.stdenv.hostPlatform) system;
                in
                {
                  inherit (self.packages.${system}) gardener;
                };
            in
            [
              overlay
            ];
        };
    };
}
