{ self, pkgs, ... }:
pkgs.testers.runNixOSTest {
  name = "check-untracked";

  nodes = {
    # `self` here is set by using specialArgs in `lib.nix`
    machine = _: {
      imports = [ self.nixosModules.default ];
      services.gardener.enable = true;
    };
  };
  # This is the test code that will check if our service is running correctly:
  testScript =
    let
      test = pkgs.writeShellScript "test" ''
        mkfifo /untracked-fifo
        touch /untracked-file
        mknod /untracked-block b 0 0
        mknod /untracked-char c 0 0
        ln -s /nonexistent /untracked-symlink
        ${pkgs.lib.getExe pkgs.python3Minimal} -c "import socket as s; sock = s.socket(s.AF_UNIX); sock.bind('/untracked-sock')"

        gardener check-untracked > output.txt
        diff -u ${./output.golden} --label nixos/tests/check-untracked/output.golden output.txt --label nixos/tests/check-untracked/output.golden
      '';
    in
    ''
      start_all()
      machine.wait_for_unit("default.target")
      machine.succeed("${test}")
    '';
}
