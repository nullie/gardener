{
  config,
  lib,
  pkgs,
  ...
}:
{
  imports = [
    ./backup.nix
    ./home-manager.nix
    ./system-modules.nix
    ./user-modules.nix
  ];

  options.services.gardener =
    let
      inherit (lib) types;
      moduleConfig =
        absolute:
        let
          pathList = lib.mkOption {
            type = types.listOf (
              types.pathWith {
                inherit absolute;
                inStore = false;
              }
            );

            default = [ ];
          };

          paths = types.submodule {
            options = {
              directories = pathList;
              files = pathList;
              symlinks = pathList;
            };
          };
        in
        types.submodule {
          options = {
            data = lib.mkOption {
              type = types.nullOr paths;
              default = null;
            };
            cache = lib.mkOption {
              type = types.nullOr paths;
              default = null;
            };
            ephemeral = lib.mkOption {
              type = types.nullOr paths;
              default = null;
            };
          };
        };
      systemModuleConfig = moduleConfig true;
      userModuleConfig = moduleConfig false;
    in
    {
      enable = lib.mkEnableOption "Enable gardener";
      availableModules = {
        system = lib.mkOption {
          type = types.attrsOf systemModuleConfig;
          default = { };
          description = "Available system modules";
        };
        user = lib.mkOption {
          type = types.attrsOf userModuleConfig;
          default = { };
          description = "Available user modules";
        };
      };
      enabledModules =
        lib.genAttrs (builtins.attrNames config.services.gardener.availableModules.system)
          (
            x:
            lib.mkOption {
              type = types.bool;
              default = false;
              description = "Enable ${x}";
            }
          );
      users = lib.mkOption {
        type = types.attrsOf (
          types.submodule (
            { name, ... }:
            {
              options = {
                adhoc = lib.mkOption {
                  type = types.attrsOf userModuleConfig;
                  default = { };
                  description = "Adhoc modules";
                };
                modules = lib.mkOption {
                  type = types.submodule {
                    options = lib.genAttrs (builtins.attrNames config.services.gardener.availableModules.user) (
                      x:
                      lib.mkOption {
                        type = types.bool;
                        default = false;
                        description = "Enable ${x}";
                      }
                    );
                  };
                  default = { };
                  description = "Generic modules";
                };
                home = lib.mkOption { readOnly = true; };
              };

              config = {
                inherit (config.users.users.${name}) home;
              };
            }
          )
        );
        default = { };
        description = "User modules";
      };
    };

  config.environment = lib.mkIf config.services.gardener.enable {
    etc."gardener.json".text = builtins.toJSON config.services.gardener;

    systemPackages = [ pkgs.gardener ];
  };
}
