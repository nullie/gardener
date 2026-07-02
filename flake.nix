{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      treefmt-nix,
    }:
    let
      forAllSystems =
        function:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
        ] (system: function nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (pkgs: rec {
        default = pkgs.rustPlatform.buildRustPackage {
          name = "gardener";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;
        };
        gardener = default;
      });

      devShells = forAllSystems (
        pkgs:
        pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            rust-analyzer
            pre-commit
            rustPackages.clippy
          ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        }
      );

      formatter = forAllSystems (
        pkgs:
        treefmt-nix.lib.mkWrapper pkgs {
          projectRootFile = "flake.nix";
          programs.nixfmt.enable = true;
          programs.keep-sorted.enable = true;
        }
      );

      nixosModules.default =
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
