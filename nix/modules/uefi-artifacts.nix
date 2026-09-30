{
  pkgs,
  self,
  nixpkgs,
}: let
  ubootVersion = "2026.07";
  pi5Uboot = pkgs.pkgsCross.aarch64-multiplatform.buildUBoot {
    version = ubootVersion;
    src = pkgs.fetchurl {
      url = "https://ftp.denx.de/pub/u-boot/u-boot-${ubootVersion}.tar.bz2";
      hash = "sha256-eOi/w4L+OI+bVaodr4xWNSKgN3ebXUw0nRQV44HxJD4=";
    };
    # Upstream's arm64 board code selects the Pi 5 C1/D0 firmware DTB.
    # https://github.com/u-boot/u-boot/blob/v2026.07/board/raspberrypi/rpi/rpi.c
    defconfig = "rpi_arm64_defconfig";
    filesToInstall = ["u-boot.bin" ".config"];
    extraConfig = ''
      CONFIG_EFI_LOADER=y
      CONFIG_EFI_VARIABLE_FILE_STORE=y
      CONFIG_FAT_WRITE=y
      CONFIG_FAT_RENAME=y
      CONFIG_BOOTMETH_EFILOADER=y
    '';
    postBuild = ''
      for setting in EFI_LOADER EFI_VARIABLE_FILE_STORE FAT_WRITE FAT_RENAME BOOTMETH_EFILOADER; do
        grep -qx "CONFIG_$setting=y" .config || {
          echo "Required UEFI capability missing: CONFIG_$setting" >&2
          exit 1
        }
      done
    '';
  };
  piFirmwareVersion = "1.20260915";
  piFirmware = pkgs.fetchzip {
    url = "https://github.com/raspberrypi/firmware/archive/refs/tags/${piFirmwareVersion}.tar.gz";
    hash = "sha256-vZxjq5icaVdlQUn5saJhJskW59xxr8IBO9iRULO6ppg=";
  };
  piFirmwareBoot = "${piFirmware}/boot";
  pi5Firmware = pkgs.runCommand "bootswain-pi5-uefi-boot-files-${ubootVersion}" {} ''
    mkdir -p $out
    cp ${pi5Uboot}/u-boot.bin $out/u-boot.bin
    cp ${pi5Uboot}/.config $out/u-boot.config
    # Include both silicon revisions rather than forcing a C1 DTB onto D0.
    for dtb in bcm2712-rpi-5-b.dtb bcm2712-d-rpi-5-b.dtb bcm2712d0-rpi-5-b.dtb; do
      test -s ${piFirmwareBoot}/$dtb
      cp ${piFirmwareBoot}/$dtb $out/$dtb
    done
    cp -r ${piFirmwareBoot}/overlays $out/overlays
    cp ${piFirmwareBoot}/LICENCE.broadcom $out/LICENCE.broadcom
    cat > $out/config.txt <<'EOF'
    [pi5]
    arm_64bit=1
    kernel=u-boot.bin
    enable_uart=1
    [all]
    EOF
  '';
  pi5Image = pkgs.runCommand "bootswain-pi5-boot-partition.img" {nativeBuildInputs = [pkgs.dosfstools pkgs.mtools];} ''
    truncate -s 512M $out
    mkfs.vfat -F 32 -n BOOTSWAIN $out
    mcopy -i $out -s ${pi5Firmware}/* ::/
  '';
  rock5aImage = pkgs.fetchurl {
    url = "https://github.com/edk2-porting/edk2-rk3588/releases/download/v1.1/rock-5a_UEFI_Release_v1.1.img";
    hash = "sha256-8NF5akwpf7RlAT27YKuh+PmZ7oy5blyLIrm2emYi1f8=";
  };
  revision = self.rev or self.dirtyRev or "working-tree";
  mkBundle = {
    board,
    release,
    image,
    kind,
    filename,
    sources,
    layout,
    notes,
  }:
    pkgs.runCommand "bootswain-${board}-${release}-bundle" {nativeBuildInputs = [pkgs.jq pkgs.coreutils];} ''
      mkdir -p $out
      cp ${image} $out/${filename}
      size=$(stat -c %s $out/${filename})
      hash=$(sha256sum $out/${filename} | cut -d ' ' -f1)
      jq -n --argjson sources '${builtins.toJSON sources}' \
        --argjson layout '${builtins.toJSON layout}' \
        --arg hash "$hash" --argjson size "$size" \
        '${builtins.toJSON {
        schema_version = 1;
        inherit board release;
        boot_policy = {
          default_order = ["sd" "usb" "nvme"];
          preferred_protocols = ["uefi"];
          no_bootable_media_behavior = "firmware console; external rescue media";
        };
        environment_policy = {
          persistent = true;
          location = null;
          stale_environment_safe = false;
          inherit notes;
        };
        validation_claims = [
          {
            target = "sd";
            protocols = ["uefi"];
            tested = false;
            scenarios = [];
            notes = "Artifact packaging only. EFI handoff, runtime variables, boot counting, watchdog reset, power cuts, and external rescue require this board.";
          }
        ];
        unsupported_paths = ["production qualification" "automatic firmware rollback" "remote recovery before firmware/network startup"];
      }} + {sources: $sources, storage_layout: $layout,
          artifacts: [{kind: "${kind}", path: "${filename}", compression: "none", size_bytes: $size, sha256: $hash, required_device_size_bytes: $size, flashable: false}]}' \
        > $out/release.json
      (cd $out; sha256sum ${filename} release.json > SHA256SUMS)
    '';
  pi5Bundle = mkBundle {
    board = "raspberry-pi-5";
    release = "${ubootVersion}-pi5-uefi-rc.0";
    image = pi5Image;
    kind = "boot-partition-image";
    filename = "pi5.boot-partition.img";
    sources = {
      u_boot = "v${ubootVersion}";
      bootswain = revision;
      nixpkgs = nixpkgs.rev;
      raspberry_pi_firmware = piFirmwareVersion;
    };
    layout = {
      spi_firmware_size_bytes = null;
      environment_offset_bytes = null;
      environment_size_bytes = null;
      shared_storage_firmware_partition = "Pi firmware FAT partition; separate OS ESP required";
    };
    # https://github.com/u-boot/u-boot/blob/v2026.07/lib/efi_loader/Kconfig
    notes = "U-Boot stores variables in /ubootefi.var on the ESP at firmware time; runtime SetVariable persistence is not provided. Pi EEPROM is pre-existing and must be qualified separately.";
  };
  rock5aBundle = mkBundle {
    board = "rock-5a";
    release = "edk2-v1.1-rock5a-rc.0";
    image = rock5aImage;
    kind = "uefi-firmware";
    filename = "rock-5a_UEFI_Release_v1.1.img";
    sources = {
      u_boot = "not-used";
      edk2 = "edk2-rk3588/v1.1";
      bootswain = revision;
      nixpkgs = nixpkgs.rev;
    };
    layout = {
      spi_firmware_size_bytes = null;
      environment_offset_bytes = 8126464;
      environment_size_bytes = 196608;
      shared_storage_firmware_partition = "Preserve upstream reserved GPT region; OS ESP begins at or beyond 16 MiB";
    };
    # https://github.com/edk2-porting/edk2-rk3588/tree/v1.1#flash-layout
    notes = "Select Device Tree/Mainline for Linux. Variable store at 0x7c0000..0x7effff is excluded from the firmware image and must survive updates. Shared SD/eMMC runtime variable access is limited; qualify the actual storage arrangement.";
  };
in {
  inherit pi5Bundle rock5aBundle;
  packages = {
    pi5-uboot = pi5Uboot;
    pi5-uefi-boot-files = pi5Firmware;
    pi5-boot-partition-image = pi5Image;
    pi5-release-bundle = pi5Bundle;
    rock5a-uefi-firmware = rock5aImage;
    rock5a-release-bundle = rock5aBundle;
  };
  checks.uefi-artifacts = pkgs.runCommand "bootswain-uefi-artifacts-check" {nativeBuildInputs = [pkgs.jq pkgs.mtools];} ''
    grep -qx 'kernel=u-boot.bin' ${pi5Firmware}/config.txt
    mdir -i ${pi5Image} ::/u-boot.bin
    mdir -i ${pi5Image} ::/bcm2712-rpi-5-b.dtb
    mdir -i ${pi5Image} ::/bcm2712-d-rpi-5-b.dtb
    mdir -i ${pi5Image} ::/bcm2712d0-rpi-5-b.dtb
    for bundle in ${pi5Bundle} ${rock5aBundle}; do
      jq -e '.schema_version == 1 and (.artifacts | length) == 1 and .artifacts[0].flashable == false and .validation_claims[0].tested == false' "$bundle/release.json"
      (cd "$bundle"; sha256sum -c SHA256SUMS)
    done
    touch $out
  '';
}
