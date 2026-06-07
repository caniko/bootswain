{self}: {
  config,
  lib,
  pkgs,
  crossbowCrossPkgs ? null,
  ...
}: let
  cfg = config.boot.bootswain.raspberryPi3BPlus;
  firmwarePackageSystem =
    if crossbowCrossPkgs != null
    then crossbowCrossPkgs.stdenv.buildPlatform.system
    else pkgs.stdenv.hostPlatform.system;
  defaultFirmwarePackage = self.packages.${firmwarePackageSystem}.raspberrypi3bplus-firmware;
  bootFiles = pkgs.buildPackages.runCommand "bootswain-raspberrypi3bplus-boot-files" {} ''
    mkdir -p "$out"
    cp ${cfg.firmwarePackage}/* "$out/"
  '';
in {
  options.boot.bootswain.raspberryPi3BPlus = {
    enable = lib.mkEnableOption "bootswain Raspberry Pi 3 B+ boot integration";

    bootMountPoint = lib.mkOption {
      type = lib.types.str;
      default = "/boot";
      description = ''
        Mounted FAT boot filesystem where bootswain Raspberry Pi 3 B+ firmware files are installed.
      '';
    };

    firmwarePackage = lib.mkOption {
      type = lib.types.package;
      default = defaultFirmwarePackage;
      defaultText = lib.literalExpression "bootswain.packages.\${pkgs.stdenv.hostPlatform.system}.raspberrypi3bplus-firmware";
      description = ''
        Bootswain Raspberry Pi 3 B+ firmware package containing Raspberry Pi boot firmware,
        config.txt, the Pi 3 B+ device tree, and u-boot-rpi3.bin.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.systemd-boot.enable = lib.mkDefault false;
    boot.loader.generic-extlinux-compatible.enable = lib.mkDefault true;

    system.build.bootswainRaspberryPi3BPlusBootFiles = bootFiles;

    system.activationScripts.bootswainRaspberryPi3BPlusBootFiles = {
      deps = ["specialfs"];
      text = ''
        bootswain_boot_mount=${lib.escapeShellArg cfg.bootMountPoint}
        if [ ! -d "$bootswain_boot_mount" ]; then
          echo "bootswain Raspberry Pi 3 B+ boot mount does not exist: $bootswain_boot_mount" >&2
          exit 1
        fi
        install -m 0644 ${bootFiles}/bootcode.bin "$bootswain_boot_mount/bootcode.bin"
        install -m 0644 ${bootFiles}/config.txt "$bootswain_boot_mount/config.txt"
        install -m 0644 ${bootFiles}/bcm2710-rpi-3-b-plus.dtb "$bootswain_boot_mount/bcm2710-rpi-3-b-plus.dtb"
        install -m 0644 ${bootFiles}/u-boot-rpi3.bin "$bootswain_boot_mount/u-boot-rpi3.bin"
        for file in ${bootFiles}/start*.elf ${bootFiles}/fixup*.dat; do
          install -m 0644 "$file" "$bootswain_boot_mount/$(basename "$file")"
        done
      '';
    };
  };
}
