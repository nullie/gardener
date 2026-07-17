{ config, lib, ... }:
{
  services.gardener.availableModules.system = {
    nixos =
      let
        isKnownSymlink = x: x.target == "gardener.json" || x.target == "systemd/system";
        entryType =
          x:
          # Checking source on some files can lead to recursion, so use known symlinks to filter those
          if isKnownSymlink x then
            "symlink"
          else if lib.strings.hasInfix "*" x.source then
            "directory"
          else if x.mode == "symlink" || x.mode == "direct-symlink" then
            "symlink"
          else
            "file";
        etcPaths =
          t:
          map (x: "/etc/${x.target}") (
            builtins.filter (x: entryType x == t) (builtins.attrValues config.environment.etc)
          );
      in
      {
        data.directories = [
          # "/etc/bluetooth"
          # "/etc/systemd"
          # "/etc/pamd.d"
          # "/etc/modprobe.d"
          # "/etc/ssh"
          # "/etc/ssl"
          "/etc/nixos"
          # "/etc/binfmt.d"
          # "/etc/ssl"
          "/var/lib/nixos"
          # Empty
        ];
        data.files = [
          "/etc/machine-id"
          "/etc/passwd"
          # TODO: shadow and maybe other should be ephemeral
          "/etc/shadow"
          "/etc/group"
          "/etc/subuid"
          "/etc/subgid"
          # nixos/modules/services/networking/wpa_supplicant.nix
          "/etc/wpa_supplicant/imperative.conf"
        ];
        ephemeral.files = [
          # TODO: investigate, try to delete and reboot
          "/etc/.pwd.lock"
          # nixos/modules/system/etc/setup-etc.pl
          "/etc/.clean"
          "/etc/NIXOS"
        ]
        ++ etcPaths "file";
        ephemeral.symlinks = etcPaths "symlink" ++ [
          # nixos/modules/system/etc/setup-etc.pl
          "/etc/static"
          # nixos/modules/config/shells-environment.nix
          "/bin/sh"
          # nixos/modules/system/activation/activation-script.nix
          "/usr/bin/env"
        ];
        ephemeral.directories =
          etcPaths "directory"
          ++ map (x: "${if x.mountPoint == "/" then "" else x.mountPoint}/lost+found") (
            builtins.filter (x: x.fsType == "ext4") (builtins.attrValues config.fileSystems)
          )
          ++ [
            "/dev"
            "/sys"
            "/proc"
            "/tmp"
            "/var/tmp"
            # TODO: split up
            "/run"
          ];
      };
    # keep-sorted start block=yes
    accounts-daemon.data.directories = [ "/var/lib/AccountsService" ];
    bluetooth.data.directories = [ "/var/lib/bluetooth" ];
    borgmatic.data.directories = [ "/var/lib/borgmatic" ];
    cups = {
      ephemeral.files = [ "/etc/printcap" ];
      cache.directories = [ "/var/cache/cups" ];
      data.directories = [
        "/var/lib/cups"
        "/var/spool/cups"
      ];
    };
    hwclock.data.files = [ "/etc/adjtime" ];
    lastlog2 = {
      data.files = [
        "/var/log/lastlog.migrated"
        "/var/lib/lastlog/lastlog2.db"
      ];
    };
    logrotate.data.files = [ "/var/lib/logrotate.status" ];
    man.cache.directories = [ "/var/cache/man" ];
    mysql.data.directories = [ "/var/lib/mysql" ];
    network-manager.data.directories = [
      "/etc/NetworkManager/system-connections"
      "/var/lib/NetworkManager"
    ];
    nix = {
      cache.directories = [ "/nix/store" ];
      data.directories = [ "/nix/var" ];
    };
    nixos-containers.data.directories = [
      # before 22.05
      "/etc/containers"
      # after 22.05
      "/etc/nixos-containers"
      "/var/lib/nixos-containers"
    ];
    nixseparatedebuginfod.cache.directories = [ "/var/cache/nixseparatedebuginfod" ];
    nixseparatedebuginfod2 = {
      ephemeral.symlinks = [ "/var/cache/nixseparatedebuginfod2" ];
      cache.directories = [ "/var/cache/private/nixseparatedebuginfod2" ];
    };
    photoprism.data.directories = [ "/var/lib/private/photoprism" ];
    photoprism.ephemeral.symlinks = [ "/var/lib/photoprism" ];
    postgresql.data.directories = [ "/var/lib/postgresql" ];
    resolvconf.ephemeral.files = [ "/etc/resolv.conf" ];
    restic.cache.directories = lib.mapAttrsToList (
      name: _: "/var/cache/restic-backups-${name}"
    ) config.services.restic.backups;
    restic.data.files = builtins.filter (x: !builtins.isNull x) (
      builtins.concatLists (
        lib.mapAttrsToList (_: value: [
          value.passwordFile
          value.repositoryFile
          value.rcloneConfigFile
        ]) config.services.restic.backups
      )
    );
    spnavd.data.files = [
      "/etc/spnavrc"
      "/var/log/spnavd.log"
    ];
    sudo.data.directories = [ "/var/db/sudo" ];
    systemd = {
      data.directories = [
        "/var/log/journal"
        "/var/lib/systemd"
      ];
      ephemeral.files = [
        # systemd-update-done
        "/etc/.updated"
        "/var/.updated"
      ];
    };
    systemd-boot.data = {
      directories = [
        # TODO: ephemeral? belonging to nixos?
        "/boot/loader"
        "/boot/EFI/nixos"
      ];
      files = [
        # Both are created by systemd-boot install
        "/boot/EFI/systemd/systemd-bootx64.efi"
        "/boot/EFI/BOOT/BOOTX64.EFI"
      ];
    };
    # keep-sorted end
  };
}
