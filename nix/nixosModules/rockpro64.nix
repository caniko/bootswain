{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.boot.bootswain.rockpro64;
  defaultFirmwarePackage = self.packages.${pkgs.stdenv.hostPlatform.system}.rockpro64-spi-firmware;
  bootFiles = pkgs.runCommand "bootswain-rockpro64-boot-files" {} ''
    mkdir -p "$out/boot"
    install -m 0644 ${cfg.firmwarePackage}/nixos-updater.boot.scr.uimg "$out/boot.scr.uimg"
    install -m 0644 ${cfg.firmwarePackage}/nixos-updater.boot.scr.uimg "$out/boot/boot.scr.uimg"
    install -m 0644 ${cfg.firmwarePackage}/u-boot-rockchip-spi.bin "$out/u-boot-rockchip-spi.bin"
    install -m 0644 ${cfg.firmwarePackage}/u-boot-rockchip-spi.bin "$out/boot/u-boot-rockchip-spi.bin"
  '';
in {
  options.boot.bootswain.rockpro64 = {
    enable = lib.mkEnableOption "bootswain ROCKPro64 boot handoff integration";

    bootMountPoint = lib.mkOption {
      type = lib.types.str;
      default = "/boot";
      description = ''
        Mounted boot filesystem where bootswain ROCKPro64 handoff artifacts are installed.
      '';
    };

    installExtlinux = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Whether to enable NixOS' generic extlinux-compatible bootloader defaults.
      '';
    };

    firmwarePackage = lib.mkOption {
      type = lib.types.package;
      default = defaultFirmwarePackage;
      defaultText = lib.literalExpression "bootswain.packages.\${pkgs.stdenv.hostPlatform.system}.rockpro64-spi-firmware";
      description = ''
        Bootswain ROCKPro64 firmware package containing the NixOS updater boot script and SPI payload.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.generic-extlinux-compatible.enable = lib.mkIf cfg.installExtlinux (lib.mkDefault true);

    system.build.bootswainRockpro64BootFiles = bootFiles;

    system.activationScripts.bootswainRockpro64BootFiles = {
      deps = ["specialfs"];
      text = ''
        bootswain_boot_mount=${lib.escapeShellArg cfg.bootMountPoint}
        if [ ! -d "$bootswain_boot_mount" ]; then
          echo "bootswain ROCKPro64 boot mount does not exist: $bootswain_boot_mount" >&2
          exit 1
        fi
        mkdir -p "$bootswain_boot_mount/boot"
        install -m 0644 ${bootFiles}/boot.scr.uimg "$bootswain_boot_mount/boot.scr.uimg"
        install -m 0644 ${bootFiles}/boot/boot.scr.uimg "$bootswain_boot_mount/boot/boot.scr.uimg"
        install -m 0644 ${bootFiles}/u-boot-rockchip-spi.bin "$bootswain_boot_mount/u-boot-rockchip-spi.bin"
        install -m 0644 ${bootFiles}/boot/u-boot-rockchip-spi.bin "$bootswain_boot_mount/boot/u-boot-rockchip-spi.bin"
      '';
    };
  };
}
