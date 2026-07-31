{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      self,
      flake-parts,
      treefmt-nix,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem =
        { pkgs, ... }:
        {

          packages = rec {
            default = pkgs.rustPlatform.buildRustPackage {
              name = "gardener";
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;
            };
            gardener = default;
          };

          devShells = {
            default = pkgs.mkShell {
              packages = with pkgs; [
                cargo
                rustc
                rustfmt
                rust-analyzer
                pre-commit
                rustPackages.clippy
              ];
              RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            };
          };

          formatter = treefmt-nix.lib.mkWrapper pkgs {
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
