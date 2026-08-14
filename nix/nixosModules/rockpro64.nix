{self}: {
  config,
  lib,
  pkgs,
  crossbowCrossPkgs ? null,
  ...
}: let
  cfg = config.boot.bootswain.rockpro64;
  effectiveOsBootProtocol =
    if cfg.installExtlinux == null
    then cfg.osBootProtocol
    else if cfg.installExtlinux
    then "extlinux"
    else "efi";
  usesEfi = effectiveOsBootProtocol == "efi";
  usesExtlinux = effectiveOsBootProtocol == "extlinux";
  firmwarePackageSystem =
    if crossbowCrossPkgs != null
    then crossbowCrossPkgs.stdenv.buildPlatform.system
    else pkgs.stdenv.hostPlatform.system;
  buildPackageSet =
    if crossbowCrossPkgs != null
    then crossbowCrossPkgs.buildPackages
    else pkgs.buildPackages;
  defaultFirmwarePackage = self.packages.${firmwarePackageSystem}.rockpro64-spi-firmware;
  removableEfiLoader = "${config.systemd.package}/lib/systemd/boot/efi/systemd-bootaa64.efi";
  nixosDtb = "rockchip/rk3399-rockpro64.dtb";
  nixosDtbPath = "${config.hardware.deviceTree.package}/${nixosDtb}";
  nextBootUsbMarker = "${cfg.bootMountPoint}/bootswain/next-boot-usb";
  rebootUsbCommand = pkgs.writeShellApplication {
    name = "bootswain-reboot-usb";
    runtimeInputs = [
      pkgs.coreutils
      config.systemd.package
    ];
    text = ''
      marker=${lib.escapeShellArg nextBootUsbMarker}
      marker_dir=$(dirname "$marker")

      if [ "$(id -u)" -ne 0 ]; then
        echo "bootswain-reboot-usb must be run as root" >&2
        exit 1
      fi

      mkdir -p "$marker_dir"
      : > "$marker"
      sync "$marker"
      sync
      systemctl reboot
    '';
  };
  bootFiles = buildPackageSet.runCommand "bootswain-rockpro64-boot-files" {} ''
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

    osBootProtocol = lib.mkOption {
      type = lib.types.enum ["efi" "extlinux" "none"];
      default = "efi";
      description = ''
        Operating-system boot protocol that bootswain prepares for ROCKPro64.
        EFI is the first-class path; extlinux remains available as an explicit fallback.
      '';
    };

    installExtlinux = lib.mkOption {
      type = lib.types.nullOr lib.types.bool;
      default = null;
      description = ''
        Deprecated compatibility option. Use boot.bootswain.rockpro64.osBootProtocol instead.
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
    warnings = lib.optional (cfg.installExtlinux != null) ''
      boot.bootswain.rockpro64.installExtlinux is deprecated; use
      boot.bootswain.rockpro64.osBootProtocol = "${effectiveOsBootProtocol}" instead.
    '';
    boot.loader.grub.enable = lib.mkDefault false;
    boot.loader.generic-extlinux-compatible.enable = lib.mkIf usesExtlinux (lib.mkDefault true);
    boot.loader.systemd-boot.enable = lib.mkIf usesEfi (lib.mkDefault true);
    boot.loader.efi.canTouchEfiVariables = lib.mkIf usesEfi (lib.mkDefault false);
    boot.loader.efi.efiSysMountPoint = lib.mkIf usesEfi (lib.mkDefault cfg.bootMountPoint);
    boot.loader.systemd-boot.extraFiles = lib.mkIf usesEfi {
      "EFI/BOOT/BOOTAA64.EFI" = lib.mkDefault removableEfiLoader;
    };

    system.build.bootswainRockpro64BootFiles = bootFiles;
    environment.systemPackages = [rebootUsbCommand];

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
        ${lib.optionalString usesEfi ''
          mkdir -p "$bootswain_boot_mount/EFI/BOOT"
          mkdir -p "$bootswain_boot_mount/dtbs/rockchip"
          install -m 0644 ${removableEfiLoader} "$bootswain_boot_mount/EFI/BOOT/BOOTAA64.EFI"
          install -m 0644 ${nixosDtbPath} "$bootswain_boot_mount/dtbs/${nixosDtb}"
        ''}
      '';
    };
  };
}
