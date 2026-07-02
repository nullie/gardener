{ config, lib, ... }:
{

  options.services.gardener.backup.exclude = lib.mkOption { readOnly = true; };

  config.services.gardener.backup.exclude =
    let
      gardenerConfig = config.services.gardener;
      moduleToPaths =
        module:
        let
          cachePaths =
            if module.cache != null then
              module.cache.directories ++ module.cache.files ++ module.cache.symlinks
            else
              [ ];
          ephemeralPaths =
            if module.ephemeral != null then
              module.ephemeral.directories ++ module.ephemeral.files ++ module.ephemeral.symlinks
            else
              [ ];
        in
        cachePaths ++ ephemeralPaths;

      systemPaths = builtins.concatLists (
        lib.mapAttrsToList (
          name: module:
          if (gardenerConfig.enabledModules.${name} or false) then (moduleToPaths module) else [ ]
        ) gardenerConfig.availableModules.system
      );

      userPaths = builtins.concatLists (
        lib.mapAttrsToList (
          userName: userConfig:
          let
            enabledModules = lib.filterAttrs (
              name: _: userConfig.modules.${name} or false
            ) gardenerConfig.availableModules.user;
            adhocModules = userConfig.adhoc;
            modules = builtins.attrValues enabledModules ++ builtins.attrValues adhocModules;
            relativePaths = builtins.concatLists (builtins.map moduleToPaths modules);
          in
          map (relativePath: "${userConfig.home}/${relativePath}") relativePaths
        ) gardenerConfig.users
      );
    in
    systemPaths ++ userPaths;
}
