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
        gardener check-untracked > output.txt
        diff ${./output.golden} output.txt
      '';
    in
    ''
      start_all()
      machine.wait_for_unit("default.target")
      machine.succeed("${test}")
    '';
}
