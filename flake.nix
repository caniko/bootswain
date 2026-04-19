{
  description = "bootswain — ROCKPro64-first host-side flash and probe tooling for stock U-Boot";

  inputs = {
    rs-harbor.url = "git+ssh://git@codeberg.org/caniko/rs-harbor.git";
    nixpkgs.follows = "rs-harbor/nixpkgs";
    rust-overlay.follows = "rs-harbor/rust-overlay";
    crane.follows = "rs-harbor/crane";
    flake-utils.follows = "rs-harbor/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rs-harbor,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        lib = pkgs.lib;
        toolchain = rs-harbor.lib.mkToolchain { inherit pkgs; };
        cross = rs-harbor.lib.mkCross { inherit pkgs system; };
        craneLib = toolchain.craneLib;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          pname = "bootswain";
          version = "0.1.0";
          strictDeps = true;
          nativeBuildInputs = [
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.udev
            pkgs.zstd
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        bootswain = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "-p bootswain-cli";
            meta = {
              description = "ROCKPro64-first host-side flash and probe tooling for stock U-Boot";
              license = with lib.licenses; [
                mit
                asl20
              ];
              mainProgram = "bootswain";
              platforms = lib.platforms.linux;
            };
          }
        );
      in
      {
        packages = {
          default = bootswain;
          inherit bootswain;
        };

        checks = {
          inherit bootswain;
          workspace-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
            }
          );
          workspace-fmt = craneLib.cargoFmt {
            inherit src;
            pname = "bootswain";
          };
          workspace-test = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--workspace";
            }
          );
        };

        devShells = rs-harbor.lib.mkDevShells {
          inherit pkgs cross;
          inherit (toolchain) craneLib;
          pkgConfigDeps = [
            pkgs.udev
            pkgs.zstd
          ];
          packages = [
            pkgs.git
            pkgs.zstd
          ];
        };
      }
    );
}

