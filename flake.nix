{
  description = "bootswain — ROCKPro64-first host-side flash and probe tooling plus firmware scaffolding";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rs-harbor = {
      url = "git+ssh://git@codeberg.org/caniko/rs-harbor.git";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay.follows = "rs-harbor/rust-overlay";
    crane.follows = "rs-harbor/crane";
    flake-utils.follows = "rs-harbor/flake-utils";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    rs-harbor,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system:
        import ./nix/per-system.nix {
          root = ./.;
          inherit self nixpkgs rs-harbor rust-overlay system;
        }
    )
    // {
      nixosModules = rec {
        rockpro64 = import ./nix/nixosModules/rockpro64.nix {inherit self;};
        rockpro64Bootable = import ./nix/nixosModules/rockpro64-bootable.nix {inherit self;};
        default = rockpro64;
      };
    };
}
