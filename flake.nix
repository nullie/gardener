{
  inputs = {
    # keep-sorted start block=yes
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
        { config, pkgs, ... }:
        {

          packages = rec {
            default = pkgs.rustPlatform.buildRustPackage {
              name = "gardener";
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;
            };
            gardener = default;
          };

          pre-commit.settings.hooks = {
            cargo-check.enable = true;
            cargo-sort.enable = true;
            clippy.enable = true;
            rustfmt.enable = true;

            nixfmt.enable = true;
            keep-sorted.enable = true;
          };

          devShells = {
            default = pkgs.mkShell {
              shellHook = config.pre-commit.shellHook;

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
                  system = prev.stdenv.hostPlatform.system;
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
