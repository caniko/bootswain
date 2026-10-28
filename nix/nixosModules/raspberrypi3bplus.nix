{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.boot.bootswain.raspberryPi3BPlus;
  defaultFirmwarePackage = self.packages.${pkgs.stdenv.buildPlatform.system}.raspberrypi3bplus-firmware;
  bootFiles = pkgs.runCommand "bootswain-raspberrypi3bplus-boot-files" {} ''
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
        Mounted FAT boot filesystem where bootswain Raspberry Pi 3 family firmware files are installed.
        If it is not a mountpoint at activation time, installation is skipped with a warning.
      '';
    };

    firmwarePackage = lib.mkOption {
      type = lib.types.package;
      default = defaultFirmwarePackage;
      defaultText = lib.literalExpression "bootswain.packages.\${pkgs.stdenv.buildPlatform.system}.raspberrypi3bplus-firmware";
      description = ''
        Bootswain Raspberry Pi 3 B+ firmware package containing Raspberry Pi boot firmware,
        config.txt, the bcm2710 Pi 3 family device trees, and u-boot-rpi3.bin.
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
        if ! ${pkgs.util-linux}/bin/mountpoint -q "$bootswain_boot_mount"; then
          echo "warning: bootswain Raspberry Pi 3 boot mount is not a mountpoint, skipping firmware install: $bootswain_boot_mount" >&2
        else
          install -m 0644 ${bootFiles}/bootcode.bin "$bootswain_boot_mount/bootcode.bin"
          install -m 0644 ${bootFiles}/config.txt "$bootswain_boot_mount/config.txt"
          install -m 0644 ${bootFiles}/u-boot-rpi3.bin "$bootswain_boot_mount/u-boot-rpi3.bin"
          for file in ${bootFiles}/bcm2710-*.dtb ${bootFiles}/start*.elf ${bootFiles}/fixup*.dat; do
            install -m 0644 "$file" "$bootswain_boot_mount/$(basename "$file")"
          done
        fi
      '';
    };
  };
}
