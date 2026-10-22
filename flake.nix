{
  description = "bootswain — ROCKPro64-first host-side flash and probe tooling plus firmware scaffolding";

  # Advertise the private macOS SDK Attic cache so darwin cross-compiles
  # substitute the realized SDK from the pin instead of rebuilding it.
  nixConfig = {
    extra-substituters = ["https://attic.candee.baby/harbor-macos-sdk"];
    extra-trusted-public-keys = [
      "harbor-macos-sdk:MLRX9qZASKwDh48UWON67cvYxfEbqvjfIZQmGwt1v1E="
    ];
  };

  inputs = {
    harbor-rs.url = "git+https://github.com/caniko/harbor-rs.git?ref=trunk";
    nixpkgs.follows = "harbor-rs/nixpkgs";
    harbor-macos-sdk-pin.url = "git+https://github.com/caniko/harbor-macos-sdk-pin.git?ref=trunk";
    rust-overlay.follows = "harbor-rs/rust-overlay";
    crane.follows = "harbor-rs/crane";
    flake-utils.url = "github:numtide/flake-utils";
    plinth = {
      url = "git+https://github.com/caniko/plinth.git?ref=refs/heads/trunk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    harbor-rs,
    harbor-macos-sdk-pin,
    plinth,
    rust-overlay,
    flake-utils,
    ...
  }: let
    mkPerSystem = {
      macosSdkStorePath ? harbor-macos-sdk-pin.storePath,
      osxSdkVersion ? harbor-macos-sdk-pin.sdkVersion,
    }: system:
      import ./nix/per-system.nix {
        root = ./.;
        inherit plinth;
        inherit self nixpkgs harbor-rs rust-overlay system macosSdkStorePath osxSdkVersion;
      };
  in
    flake-utils.lib.eachSystem ["x86_64-linux" "aarch64-linux" "aarch64-darwin"] (mkPerSystem {})
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
