{
  description = "bootswain — ROCKPro64-first host-side flash and probe tooling plus firmware scaffolding";

  # Advertise the private macOS SDK Attic cache so darwin cross-compiles
  # substitute the realized SDK from the pin instead of rebuilding it.
  nixConfig = {
    extra-substituters = ["https://attic.candee.baby/harbor-macos-sdk"];
    extra-trusted-public-keys = [
      "harbor-macos-sdk:ci7MNMkHDqdeTS4aKwzDNEJ1175AbpVUypTRjCJoHDk="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rs-harbor = {
      url = "git+https://codeberg.org/caniko/rs-harbor.git?ref=trunk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rs-harbor-macos-sdk-pin.url = "git+ssh://git@codeberg.org/caniko/rs-harbor-macos-sdk-pin.git";
    rust-overlay.follows = "rs-harbor/rust-overlay";
    crane.follows = "rs-harbor/crane";
    flake-utils.follows = "rs-harbor/flake-utils";
    plinth = {
      url = "git+https://codeberg.org/caniko/plinth.git?ref=refs/heads/trunk";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    rs-harbor,
    rs-harbor-macos-sdk-pin,
    plinth,
    rust-overlay,
    flake-utils,
    ...
  }: let
    mkPerSystem = {
      macosSdkStorePath ? rs-harbor-macos-sdk-pin.storePath,
      osxSdkVersion ? rs-harbor-macos-sdk-pin.sdkVersion,
    }: system:
      import ./nix/per-system.nix {
        root = ./.;
        inherit plinth;
        inherit self nixpkgs rs-harbor rust-overlay system macosSdkStorePath osxSdkVersion;
      };
  in
    flake-utils.lib.eachDefaultSystem (mkPerSystem {})
    // {
      lib = {
        inherit mkPerSystem;
      };

      nixosModules = rec {
        rockpro64 = import ./nix/nixosModules/rockpro64.nix {inherit self;};
        rockpro64Bootable = import ./nix/nixosModules/rockpro64-bootable.nix {inherit self;};
        raspberryPi3BPlus = import ./nix/nixosModules/raspberrypi3bplus.nix {inherit self;};
        raspberryPi3BPlusBootable = import ./nix/nixosModules/raspberrypi3bplus-bootable.nix {inherit self;};
        default = rockpro64;
      };
    };
}
