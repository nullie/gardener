(import ./lib.nix) {
  name = "from-nixos";
  nodes = {
    # `self` here is set by using specialArgs in `lib.nix`
    machine =
      { self, ... }:
      {
        imports = [ self.nixosModules.default ];
        services.gardener.enable = true;
      };
  };
  # This is the test code that will check if our service is running correctly:
  testScript = ''
    start_all()
    output = machine.succeed("gardener check-untracked")
    print(output)
  '';
}
