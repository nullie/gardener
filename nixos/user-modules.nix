{ config, lib, ... }:
{
  services.gardener.availableModules.user = {
    # keep-sorted start block=yes
    android-tools.data.directories = [ ".android" ];
    atuin.data.directories = [ ".local/share/atuin" ];
    bambu = {
      cache.directories = [ ".cache/bambu-studio" ];
      data.directories = [
        ".config/BambuStudio"
        ".local/share/bambu-studio"
      ];
    };
    bash.data.files = [ ".bash_history" ];
    borg.cache.directories = [ ".cache/borg" ];
    btop.data.files = [
      ".config/btop/btop.conf"
      ".local/state/btop.log"
    ];
    cargo.cache.directories = [ ".cargo" ];
    cava.data.directories = [ ".config/cava" ];
    dconf = {
      data.files = [ ".config/dconf/user" ];
      cache.files = [ ".cache/dconf/user" ];
    };
    dgop.data.files = [ ".config/dgop/colors.json" ];
    direnv.data.directories = [ ".local/share/direnv/allow" ];
    discord.data.directories = [ ".config/discord" ];
    dms = {
      cache.directories = [
        ".cache/dms"
        ".cache/DankMaterialShell"
      ];
      data.directories = [ ".local/state/DankMaterialShell/notepad-files" ];
      data.files = [
        ".config/DankMaterialShell/.firstlaunch"
        ".config/DankMaterialShell/.changelog-1.4"
        ".local/state/DankMaterialShell/appusage.json"
        ".local/state/DankMaterialShell/notepad-session.json"
      ];
      ephemeral.files = [
        ".config/DankMaterialShell/firefox.css"
        ".config/alacritty/dank-theme.toml"
        ".config/cosmic/com.system76.CosmicTheme.Mode/v1/is_dark"
        ".config/gtk-3.0/dank-colors.css"
        ".config/gtk-4.0/dank-colors.css"
        ".local/share/color-schemes/DankMatugen.colors"
        ".local/share/color-schemes/DankMatugenDark.colors"
        ".local/share/color-schemes/DankMatugenLight.colors"
      ];
    };
    electron.cache.directories = [ ".config/Electron" ];
    evcxr.data.directories = [ ".config/evcxr" ];
    firefox = {
      cache.directories = [ ".cache/mozilla/firefox" ];
      data.directories = [ ".config/mozilla/firefox" ];
    };
    fontconfig.cache.directories = [ ".cache/fontconfig" ];
    freecad = {
      cache.directories = [ ".cache/FreeCAD" ];
      data.directories = [
        ".config/FreeCAD"
        ".local/share/FreeCAD"
      ];
    };
    gimp = {
      cache.directories = [ ".cache/gimp" ];
      data.directories = [ ".config/GIMP" ];
    };
    gnome-desktop-thumbnailer.cache.directories = [
      ".cache/gnome-desktop-thumbnailer"
      ".cache/thumbnails"
    ];
    gnome-keyring.data.directories = [ ".local/share/keyrings" ];
    google-chrome = {
      data.directories = [ ".config/google-chrome" ];
      cache.directories = [ ".cache/google-chrome" ];
    };
    gstreamer.cache.directories = [ ".cache/gstreamer-1.0" ];
    gtk.cache.directories = [ ".cache/gtk-4.0" ];
    helix = {
      cache.directories = [ ".cache/helix" ];
      data.directories = [ ".config/helix" ];
    };
    inkscape = {
      cache.directories = [ ".cache/inkscape" ];
      data.directories = [ ".config/inkscape" ];
    };
    ipython.data.directories = [ ".ipython/profile_default" ];
    jedi.cache.directories = [ ".cache/jedi" ];
    lazygit.data.directories = [ ".local/state/lazygit" ];
    less.data.files = [
      ".lesshst"
      ".local/state/lesshst"
    ];
    libreoffice.data.directories = [ ".config/libreoffice" ];
    lua-language-server.cache.directories = [ ".cache/lua-language-server" ];
    mesa.cache.directories = [
      ".cache/mesa_shader_cache"
      ".cache/mesa_shader_cache_db"
    ];
    mkcert.data.directories = [ ".local/share/mkcert" ];
    nautilus = {
      data.directories = [
        ".config/nautilus"
        ".local/share/nautilus"
        ".local/share/Trash/files"
      ];
      data.files = [ ".local/share/recently-used.xbel" ];
    };
    neovim = {
      cache.directories = [ ".cache/nvim" ];
      data.directories = [
        ".local/state/nvim"
        ".local/share/nvim"
      ];
    };
    nix = {
      cache.directories = [
        ".cache/nix"
        ".cache/nix-index"
      ];
      data.directories = [
        ".nix-defexpr"
        ".local/state/nix/profiles"
        ".local/share/nix"
      ];
      data.symlinks = [ ".nix-profile" ];
    };
    nix-output-monitor.data.directories = [ ".local/state/nix-output-monitor" ];
    nixseparatedebuginfod2.cache.directories = [ ".cache/debuginfod_client" ];
    nss.ephemeral.directories = [ ".pki" ];
    obsidian.data.directories = [ ".config/obsidian" ];
    oh-my-zsh.cache.directories = [ ".cache/oh-my-zsh" ];
    pavucontrol.data.files = [ ".config/pavucontrol.ini" ];
    pre-commit.cache.directories = [ ".cache/pre-commit" ];
    prusa-slicer = {
      cache.directories = [ ".cache/prusa-slicer" ];
      data.directories = [
        ".config/PrusaSlicer"
        ".local/share/prusa-slicer"
      ];
    };
    psql.data.files = [ ".psql_history" ];
    pulseaudio.ephemeral.files = [
      ".config/pulse/cookie"
    ];
    python.data.files = [ ".python_history" ];
    qt = {
      cache.directories = [ ".cache/qtshadercache-x86_64-little_endian-lp64" ];
      data.files = [ ".config/QtProject.conf" ];
    };
    qualculate.data.files = [
      ".config/qalculate/qalc.cfg"
      ".local/state/qalculate/qalc.history"
    ];
    quickshell.cache.directories = [ ".cache/quickshell" ];
    qutebrowser = {
      cache.directories = [ ".cache/qutebrowser" ];
      data.directories = [
        ".config/qutebrowser"

        # history, cmd-history, blocked-hosts
        ".local/share/qutebrowser"
      ];
    };
    radv.cache.directories = [ ".cache/radv_builtin_shaders" ];
    rbw.cache.directories = [
      ".local/share/rbw"
      ".cache/rbw"
    ];
    rofi-rbw.cache.files = [
      ".cache/rofi-rbw.runcache"
      ".cache/rofi3.druncache"
    ];
    rofi.cache.files = [ ".cache/rofi-entry-history.txt" ];
    shotwell = {
      cache.directories = [ ".cache/shotwell" ];
      data.directories = [
        ".config/shotwell"
        ".local/share/shotwell"
      ];
    };
    ssh.data.directories = [ ".ssh" ];
    starship.cache.directories = [ ".cache/starship" ];
    steam = {
      cache.directories = [ ".local/share/Steam" ];
      ephemeral = {
        directories = [ ".steam" ];
        symlinks = [
          ".steampid"
          ".steampath"
        ];
      };
    };
    telegram = {
      cache.directories = [ ".cache/TelegramDesktop" ];
      data.directories = [ ".local/share/TelegramDesktop" ];
    };
    telescope-nvim.data.files = [ ".local/share/nvim/telescope_history" ];
    treefmt.cache.directories = [ ".cache/treefmt" ];
    vulkan.cache.directories = [ ".local/share/vulkan" ];
    wireplumber.data.directories = [ ".local/state/wireplumber" ];
    zoxide.data.directories = [ ".local/share/zoxide" ];
    zsh = {
      cache.directories = [ ".config/zsh" ];
      data.files = [ ".zsh_history" ];
    };
    # keep-sorted end
  };
}
