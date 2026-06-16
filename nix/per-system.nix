{
  root,
  self,
  nixpkgs,
  plinth,
  rs-harbor,
  rust-overlay,
  system,
  macosSdkStorePath ? null,
  osxSdkVersion ? "26.1",
}: let
  pkgs = import nixpkgs {
    inherit system;
    config.allowUnfreePredicate = pkg:
      builtins.elem
      (
        if pkg ? pname
        then pkg.pname
        else (builtins.parseDrvName pkg.name).name
      )
      [
        "arm-trusted-firmware-rk3399"
        "raspberrypi-firmware"
      ];
    overlays = [(import rust-overlay)];
  };
  lib = pkgs.lib;
  toolchain = rs-harbor.lib.mkToolchain {inherit pkgs;};
  cross = rs-harbor.lib.mkCross ({
      inherit pkgs system osxSdkVersion;
    }
    // lib.optionalAttrs (macosSdkStorePath != null) {
      inherit macosSdkStorePath;
    });
  craneLib = toolchain.craneLib;
  workspaceRoot = toString root;
  src = lib.cleanSourceWith {
    src = root;
    filter = path: type: let
      relativePath = lib.removePrefix "${workspaceRoot}/" (toString path);
    in
      (craneLib.filterCargoSources path type)
      || lib.hasPrefix "docs/" relativePath
      || lib.hasPrefix "firmware/" relativePath
      || lib.hasPrefix "validation/" relativePath;
  };

  artifacts = import ./modules/artifacts.nix {
    inherit pkgs;
    inherit root self nixpkgs;
  };

  workspace = import ./modules/workspace.nix {
    inherit pkgs lib craneLib src;
    inherit
      (artifacts)
      qemuArm64Uboot
      validationDir
      raspberryPi3BPlusReleaseBundle
      raspberryPi3BPlusReleaseCandidate
      rockpro64ExperimentalRelease
      rockpro64ReleaseBundleExperimental
      ;
  };

  flash = import ./modules/flash.nix {
    inherit pkgs;
    inherit (workspace) bootswain;
    inherit
      (artifacts)
      raspberryPi3BPlusReleaseBundle
      rockpro64ReleaseBundle
      rockpro64ReleaseBundleExperimental
      ;
  };

  sites = import ./modules/sites.nix {
    inherit pkgs lib root;
  };
  website = plinth.lib.${system}.mkProjectSite {
    pname = "bootswain-website";
    domain = "bootswain.tartanoglu.com";
    configPath = root + /website/plinth-project.toml;
    docsPackage = sites.packages.site or null;
  };

  shells = import ./modules/shells.nix {
    inherit rs-harbor pkgs cross toolchain;
    plinthProject = plinth.packages.${system}.plinth-project;
  };

  nixosModuleCheck = let
    fakeFirmwarePackage = pkgs.runCommand "bootswain-rockpro64-fake-firmware" {} ''
      mkdir -p "$out"
      printf '%s\n' "fake updater script" > "$out/nixos-updater.boot.scr.uimg"
      printf '%s\n' "fake spi payload" > "$out/u-boot-rockchip-spi.bin"
    '';
    fakeRaspberryPi3BPlusFirmwarePackage = pkgs.runCommand "bootswain-raspberrypi3bplus-fake-firmware" {} ''
      mkdir -p "$out"
      printf '%s\n' "fake bootcode" > "$out/bootcode.bin"
      printf '%s\n' "fake start" > "$out/start.elf"
      printf '%s\n' "fake start cd" > "$out/start_cd.elf"
      printf '%s\n' "fake fixup" > "$out/fixup.dat"
      printf '%s\n' "fake fixup cd" > "$out/fixup_cd.dat"
      printf '%s\n' "fake dtb" > "$out/bcm2710-rpi-3-b-plus.dtb"
      printf '%s\n' "fake uboot" > "$out/u-boot-rpi3.bin"
      printf '%s\n' "kernel=u-boot-rpi3.bin" > "$out/config.txt"
    '';
    mkTestSystem = testSystem:
      nixpkgs.lib.nixosSystem {
        system = testSystem;
        modules = [
          self.nixosModules.rockpro64
          (
            {...}: {
              boot.bootswain.rockpro64 = {
                enable = true;
                firmwarePackage = fakeFirmwarePackage;
              };
              fileSystems."/" = {
                device = "none";
                fsType = "tmpfs";
              };
              system.stateVersion = "26.05";
            }
          )
        ];
      };
    bootableConfig = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        self.nixosModules.rockpro64Bootable
        (
          {...}: {
            boot.bootswain.rockpro64.firmwarePackage = fakeFirmwarePackage;
            fileSystems."/" = {
              device = "none";
              fsType = "tmpfs";
            };
            system.stateVersion = "26.05";
          }
        )
      ];
    };
    raspberryPi3BPlusConfig = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        self.nixosModules.raspberryPi3BPlus
        (
          {...}: {
            boot.bootswain.raspberryPi3BPlus = {
              enable = true;
              firmwarePackage = fakeRaspberryPi3BPlusFirmwarePackage;
            };
            fileSystems."/" = {
              device = "none";
              fsType = "tmpfs";
            };
            system.stateVersion = "26.05";
          }
        )
      ];
    };
    raspberryPi3BPlusBootableConfig = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        self.nixosModules.raspberryPi3BPlusBootable
        (
          {...}: {
            boot.bootswain.raspberryPi3BPlus.firmwarePackage = fakeRaspberryPi3BPlusFirmwarePackage;
            fileSystems."/" = {
              device = "none";
              fsType = "tmpfs";
            };
            system.stateVersion = "26.05";
          }
        )
      ];
    };
    currentSystemConfig = mkTestSystem system;
    aarch64Config = mkTestSystem "aarch64-linux";
    aarch64EvalSummary = pkgs.writeText "bootswain-rockpro64-aarch64-nixos-eval.txt" ''
      bootFiles=${aarch64Config.config.system.build.bootswainRockpro64BootFiles.name}
      grub=${
        if aarch64Config.config.boot.loader.grub.enable
        then "true"
        else "false"
      }
      extlinux=${
        if aarch64Config.config.boot.loader.generic-extlinux-compatible.enable
        then "true"
        else "false"
      }
      systemdBoot=${
        if aarch64Config.config.boot.loader.systemd-boot.enable
        then "true"
        else "false"
      }
      rockpro64DtbExists=${
        if builtins.pathExists "${aarch64Config.config.hardware.deviceTree.package}/rockchip/rk3399-rockpro64.dtb"
        then "true"
        else "false"
      }
      rockpro64DtbActivation=${
        if lib.hasInfix "dtbs/rockchip/rk3399-rockpro64.dtb" aarch64Config.config.system.activationScripts.bootswainRockpro64BootFiles.text
        then "true"
        else "false"
      }
      rebootUsbCommand=${
        if lib.any (pkg: lib.hasInfix "bootswain-reboot-usb" (toString pkg)) aarch64Config.config.environment.systemPackages
        then "true"
        else "false"
      }
      bootableHostPlatform=${bootableConfig.config.nixpkgs.hostPlatform.system}
      bootableEnabled=${
        if bootableConfig.config.boot.bootswain.rockpro64.enable
        then "true"
        else "false"
      }
      bootableConsole=${builtins.concatStringsSep " " bootableConfig.config.boot.kernelParams}
      raspberryPi3BPlusBootFiles=${raspberryPi3BPlusConfig.config.system.build.bootswainRaspberryPi3BPlusBootFiles.name}
      raspberryPi3BPlusGrub=${
        if raspberryPi3BPlusConfig.config.boot.loader.grub.enable
        then "true"
        else "false"
      }
      raspberryPi3BPlusExtlinux=${
        if raspberryPi3BPlusConfig.config.boot.loader.generic-extlinux-compatible.enable
        then "true"
        else "false"
      }
      raspberryPi3BPlusSystemdBoot=${
        if raspberryPi3BPlusConfig.config.boot.loader.systemd-boot.enable
        then "true"
        else "false"
      }
      raspberryPi3BPlusBootableHostPlatform=${raspberryPi3BPlusBootableConfig.config.nixpkgs.hostPlatform.system}
      raspberryPi3BPlusBootableEnabled=${
        if raspberryPi3BPlusBootableConfig.config.boot.bootswain.raspberryPi3BPlus.enable
        then "true"
        else "false"
      }
      raspberryPi3BPlusBootableConsole=${builtins.concatStringsSep " " raspberryPi3BPlusBootableConfig.config.boot.kernelParams}
    '';
  in
    pkgs.runCommand "bootswain-rockpro64-nixos-module-check" {} ''
      boot_files=${currentSystemConfig.config.system.build.bootswainRockpro64BootFiles}
      rpi_boot_files=${raspberryPi3BPlusConfig.config.system.build.bootswainRaspberryPi3BPlusBootFiles}

      test -s "$boot_files/boot.scr.uimg"
      test -s "$boot_files/boot/boot.scr.uimg"
      test -s "$boot_files/u-boot-rockchip-spi.bin"
      test -s "$boot_files/boot/u-boot-rockchip-spi.bin"
      cmp "$boot_files/boot.scr.uimg" "$boot_files/boot/boot.scr.uimg"
      cmp "$boot_files/u-boot-rockchip-spi.bin" "$boot_files/boot/u-boot-rockchip-spi.bin"
      test -s "$rpi_boot_files/bootcode.bin"
      test -s "$rpi_boot_files/config.txt"
      test -s "$rpi_boot_files/bcm2710-rpi-3-b-plus.dtb"
      test -s "$rpi_boot_files/u-boot-rpi3.bin"
      test -s "$rpi_boot_files/start.elf"
      test -s "$rpi_boot_files/fixup.dat"

      grep -q '^bootFiles=bootswain-rockpro64-boot-files$' ${aarch64EvalSummary}
      grep -q '^grub=false$' ${aarch64EvalSummary}
      grep -q '^extlinux=false$' ${aarch64EvalSummary}
      grep -q '^systemdBoot=true$' ${aarch64EvalSummary}
      grep -q '^rockpro64DtbExists=true$' ${aarch64EvalSummary}
      grep -q '^rockpro64DtbActivation=true$' ${aarch64EvalSummary}
      grep -q '^rebootUsbCommand=true$' ${aarch64EvalSummary}
      grep -q '^bootableHostPlatform=aarch64-linux$' ${aarch64EvalSummary}
      grep -q '^bootableEnabled=true$' ${aarch64EvalSummary}
      grep -q '^bootableConsole=.*console=ttyS2,115200n8' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusBootFiles=bootswain-raspberrypi3bplus-boot-files$' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusGrub=false$' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusExtlinux=true$' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusSystemdBoot=false$' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusBootableHostPlatform=aarch64-linux$' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusBootableEnabled=true$' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusBootableConsole=.*console=ttyAMA0,115200n8' ${aarch64EvalSummary}
      grep -q '^raspberryPi3BPlusBootableConsole=.*console=tty0' ${aarch64EvalSummary}

      mkdir -p "$out"
      cp ${aarch64EvalSummary} "$out/aarch64-eval.txt"
      printf '%s\n' "bootswain ROCKPro64 NixOS module check passed" > "$out/result"
    '';
in {
  packages =
    workspace.packages
    // artifacts.packages
    // sites.packages
    // {
      inherit website;
      site = website;
    };
  apps =
    flash.apps
    // {
      deploy-pages = plinth.lib.${system}.mkDeployPagesApp {
        domain = "bootswain.tartanoglu.com";
      };
    };
  checks =
    workspace.checks
    // artifacts.checks
    // flash.checks
    // {
      rockpro64-nixos-module = nixosModuleCheck;
    };
  devShells = shells.devShells;
}
