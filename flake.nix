{
  description = "bootswain — ROCKPro64-first host-side flash and probe tooling plus firmware scaffolding";

  inputs = {
    harbor-rs.url = "git+https://github.com/caniko/harbor-rs.git?ref=trunk";
    nixpkgs.follows = "harbor-rs/nixpkgs";
    rust-overlay.follows = "harbor-rs/rust-overlay";
    crane.follows = "harbor-rs/crane";
    flake-utils.url = "github:numtide/flake-utils";
    treefmt-nix.follows = "harbor-rs/treefmt-nix";
    plinth = {
      url = "git+https://github.com/caniko/plinth.git?ref=trunk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    harbor-rs,
    plinth,
    rust-overlay,
    flake-utils,
    treefmt-nix,
    ...
  }: let
    # Optional macOS SDK (store path + version) for darwin cross builds; omitted by default.
    mkPerSystem = {
      macosSdkStorePath ? null,
      osxSdkVersion ? "26.1",
    }: system:
      import ./nix/per-system.nix {
        root = ./.;
        inherit plinth treefmt-nix;
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
