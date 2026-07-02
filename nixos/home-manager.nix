{ config, lib, ... }:
{
  services.gardener.users = builtins.mapAttrs (userName: hmConfig: {
    adhoc.home-manager = {
      cache.directories = [ ".local/state/home-manager/gcroots" ];

      ephemeral =
        let
          homeFiles = builtins.attrValues hmConfig.home.file;
          recursive = lib.lists.partition (x: x.recursive) homeFiles;
        in
        {
          directories = map (x: x.target) recursive.right;
          symlinks = map (x: x.target) recursive.wrong;
        };
    };
  }) config.home-manager.users;
}
