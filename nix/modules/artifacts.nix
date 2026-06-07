{
  pkgs,
  root,
  self,
  nixpkgs,
}: let
  lock = builtins.fromJSON (builtins.readFile (root + "/flake.lock"));
  nixpkgsRevision =
    if lock.nodes.nixpkgs.locked ? rev
    then lock.nodes.nixpkgs.locked.rev
    else nixpkgs.outPath;
  bootswainRevision =
    if self ? rev
    then self.rev
    else if self ? dirtyRev
    then self.dirtyRev
    else "dirty-worktree";

  rockpro64FirmwareDir = builtins.path {
    path = root + "/firmware/rockpro64";
    name = "rockpro64-firmware";
  };
  firmwareLibDir = builtins.path {
    path = root + "/firmware/lib";
    name = "bootswain-firmware-lib";
  };
  validationDir = builtins.path {
    path = root + "/validation";
    name = "bootswain-validation-fixtures";
  };

  qemuArm64Uboot = pkgs.pkgsCross.aarch64-multiplatform.ubootQemuAarch64;
  rockpro64TfA = pkgs.pkgsCross.aarch64-multiplatform.armTrustedFirmwareRK3399;
  raspberryPi3BPlusUboot = pkgs.pkgsCross.aarch64-multiplatform.ubootRaspberryPi3_64bit;
  raspberryPi3BPlusFirmware = pkgs.raspberrypifw;
  raspberryPi3BPlusFirmwareBoot = "${raspberryPi3BPlusFirmware}/share/raspberrypi/boot";

  mkRenderedBootCmd = {
    name,
    fragment,
  }:
    pkgs.runCommand name {} ''
      : > "$out"
      while IFS= read -r line || [ -n "$line" ]; do
        if [ "$line" = "@@BOOTSWAIN_BOARD_FRAGMENT@@" ]; then
          cat ${fragment} >> "$out"
        else
          printf '%s\n' "$line" >> "$out"
        fi
      done < ${firmwareLibDir}/generic-boot-menu.cmd
    '';

  mkBootScript = {
    name,
    bootCmd,
  }:
    pkgs.runCommand name {nativeBuildInputs = [pkgs.ubootTools];} ''
      cp ${bootCmd} boot.cmd
      mkimage -A arm64 -T script -C none -d boot.cmd "$out"
    '';

  rockpro64FirmwareFragment = pkgs.runCommand "rockpro64-firmware-fragment.cmd" {} ''
        sed \
          -e 's/setenv bootswain_status .*/setenv bootswain_status release-candidate/' \
          -e 's/setenv bootswain_validation .*/setenv bootswain_validation hardware-required/' \
          -e 's/setenv bootswain_release_stage .*/setenv bootswain_release_stage release-candidate/' \
          -e 's/setenv bootswain_release_channel .*/setenv bootswain_release_channel release-candidate/' \
          -e 's/setenv bootswain_runtime_role .*/setenv bootswain_runtime_role firmware/' \
          -e "s/setenv bootswain_menu_title .*/setenv bootswain_menu_title 'bootswain ROCKPro64 firmware'/" \
          -e "s/setenv bootswain_ready_message .*/setenv bootswain_ready_message 'ROCKPro64 firmware menu ready'/" \
          ${rockpro64FirmwareDir}/boot.cmd \
          | sed '/^if test -n ".*bootswain_chainload_devtype/,$d' > "$out"
        cat >> "$out" <<'EOF'

    setenv bootswain_flash_spi 'echo "SPI flashing is available only from the installer image"; false'
    setenv bootswain_erase_spi 'echo "SPI erase is available only from the installer image"; false'
    EOF
  '';

  rockpro64FirmwareEnv = pkgs.runCommand "rockpro64-firmware.env" {} ''
    sed \
      -e 's/^bootswain_status=.*/bootswain_status=release-candidate/' \
      -e 's/^bootswain_validation=.*/bootswain_validation=hardware-required/' \
      -e 's/^bootswain_release_stage=.*/bootswain_release_stage=release-candidate/' \
      -e 's/^bootswain_release_channel=.*/bootswain_release_channel=release-candidate/' \
      -e 's/^bootswain_runtime_role=.*/bootswain_runtime_role=firmware/' \
      -e 's/^bootswain_menu_title=.*/bootswain_menu_title=bootswain ROCKPro64 firmware/' \
      -e 's/^bootmenu_title=.*/bootmenu_title=bootswain ROCKPro64 firmware/' \
      -e 's/^bootswain_ready_message=.*/bootswain_ready_message=ROCKPro64 firmware menu ready/' \
      -e 's/^bootswain_flash_spi=.*/bootswain_flash_spi=echo "SPI flashing is available only from the installer image"; false/' \
      -e 's/^bootswain_erase_spi=.*/bootswain_erase_spi=echo "SPI erase is available only from the installer image"; false/' \
      -e 's/^bootswain_recovery_hint=.*/bootswain_recovery_hint=Release-candidate firmware; use the serial console for recovery./' \
      -e 's/^bootswain_installer_hint=.*/bootswain_installer_hint=Full Phase 0 claims require imported hardware validation evidence./' \
      ${rockpro64FirmwareDir}/uboot.env > "$out"
  '';

  mkRockpro64Uboot = {
    role,
    envFile,
  }:
    (pkgs.pkgsCross.aarch64-multiplatform.ubootRockPro64.override {
      extraConfig = ''
        ${builtins.readFile "${rockpro64FirmwareDir}/u-boot.config.fragment"}
        CONFIG_ENV_USE_DEFAULT_ENV_TEXT_FILE=y
        CONFIG_ENV_DEFAULT_ENV_TEXT_FILE="${envFile}"
      '';
      filesToInstall = [
        "u-boot.itb"
        "idbloader.img"
        "u-boot-rockchip.bin"
        "u-boot-rockchip-spi.bin"
      ];
    }).overrideAttrs
    (old: {
      pname = "${old.pname}-${role}";
      nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.buildPackages.xxd];
    });

  rockpro64InstallerBootCmd = mkRenderedBootCmd {
    name = "rockpro64-installer-boot.cmd";
    fragment = "${rockpro64FirmwareDir}/boot.cmd";
  };
  rockpro64FirmwareBootCmd = mkRenderedBootCmd {
    name = "rockpro64-firmware-boot.cmd";
    fragment = rockpro64FirmwareFragment;
  };
  rockpro64InstallerBootScript = mkBootScript {
    name = "rockpro64-installer-boot.scr.uimg";
    bootCmd = rockpro64InstallerBootCmd;
  };
  rockpro64FirmwareBootScript = mkBootScript {
    name = "rockpro64-firmware-boot.scr.uimg";
    bootCmd = rockpro64FirmwareBootCmd;
  };
  rockpro64NixosUpdaterBootCmd = pkgs.writeText "rockpro64-nixos-updater-boot.cmd" ''
    # bootswain ROCKPro64 NixOS boot handoff.

    setenv bootswain_board pine64-rockpro64
    setenv bootswain_soc rockchip-rk3399
    setenv bootswain_runtime_role nixos-updater
    setenv bootswain_release_channel release-candidate

    if test -z "''${fdtfile}"; then
    	setenv fdtfile rockchip/rk3399-rockpro64.dtb
    fi
    if test -z "''${scriptaddr}"; then
    	setenv scriptaddr 0x00c00000
    fi
    if test -z "''${kernel_addr_r}"; then
    	setenv kernel_addr_r 0x02000000
    fi
    if test -z "''${fdt_addr_r}"; then
    	setenv fdt_addr_r 0x12000000
    fi
    if test -z "''${ramdisk_addr_r}"; then
    	setenv ramdisk_addr_r 0x12180000
    fi
    if test -z "''${devtype}"; then
    	setenv devtype mmc
    fi
    if test -z "''${devnum}"; then
    	setenv devnum 1
    fi
    if test -z "''${distro_bootpart}"; then
    	setenv distro_bootpart 1
    fi

    setenv bootswain_spi_source_devtype "''${devtype}"
    setenv bootswain_spi_source_devnum "''${devnum}"
    setenv bootswain_spi_source_bootpart "''${distro_bootpart}"
    setenv bootswain_spi_image /u-boot-rockchip-spi.bin
    setenv bootswain_spi_image_fallback /boot/u-boot-rockchip-spi.bin
    setenv bootswain_spi_size 0x1000000
    setenv bootswain_spi_max_payload_size 0x1000000
    setenv bootswain_spi_verify_addr 0x18000000
    setenv bootswain_expected_compatible pine64,rockpro64
    setenv bootswain_expected_compatible_v2_1 pine64,rockpro64-v2.1

    setenv bootswain_boot_target_emmc mmc0
    setenv bootswain_boot_target_sd mmc1
    setenv bootswain_boot_target_usb usb0
    setenv bootswain_boot_target_nvme nvme0
    setenv bootswain_boot_os_bootmeths "efi extlinux"
    setenv bootswain_prepare_os_bootmeths 'echo "Selecting OS boot methods: ''${bootswain_boot_os_bootmeths}"; bootmeth order "''${bootswain_boot_os_bootmeths}"'
    setenv bootswain_efi_loader /EFI/BOOT/BOOTAA64.EFI
    setenv bootswain_efi_bootpart 1
    setenv bootswain_next_boot_usb_devtype mmc
    setenv bootswain_next_boot_usb_devnum 0
    setenv bootswain_next_boot_usb_bootpart 1
    setenv bootswain_next_boot_usb_marker /bootswain/next-boot-usb
    setenv bootswain_boot_prepare_emmc 'true'
    setenv bootswain_boot_prepare_sd 'true'
    setenv bootswain_boot_prepare_usb 'usb start; true'
    setenv bootswain_boot_prepare_nvme 'pci enum; nvme scan; true'
    setenv bootswain_boot_normal_order 'if run bootswain_boot_emmc; then true; elif run bootswain_boot_sd; then true; elif run bootswain_boot_usb; then true; elif run bootswain_boot_nvme; then true; else echo "No bootable media"; false; fi'
    setenv bootswain_boot_usb_oneshot_order 'if run bootswain_boot_usb; then true; elif run bootswain_boot_emmc; then true; elif run bootswain_boot_sd; then true; elif run bootswain_boot_nvme; then true; else echo "No bootable media"; false; fi'
    setenv bootswain_consume_next_boot_usb 'if fatls ''${bootswain_next_boot_usb_devtype} ''${bootswain_next_boot_usb_devnum}:''${bootswain_next_boot_usb_bootpart} ''${bootswain_next_boot_usb_marker}; then echo "Consuming one-shot USB boot marker"; if fatrm ''${bootswain_next_boot_usb_devtype} ''${bootswain_next_boot_usb_devnum}:''${bootswain_next_boot_usb_bootpart} ''${bootswain_next_boot_usb_marker}; then true; else echo "Failed to remove one-shot USB boot marker"; false; fi; else false; fi'
    setenv bootswain_boot_auto 'if run bootswain_consume_next_boot_usb; then echo "One-shot USB boot requested"; run bootswain_boot_usb_oneshot_order; else run bootswain_boot_normal_order; fi'
    setenv bootswain_boot_direct_efi 'echo "Trying direct EFI handoff"; echo "EFI target: ''${bootswain_efi_target}"; echo "EFI device: ''${bootswain_efi_devtype} ''${bootswain_efi_devnum}:''${bootswain_efi_bootpart}"; echo "EFI loader: ''${bootswain_efi_loader}"; bootmeth list -a; if fatls ''${bootswain_efi_devtype} ''${bootswain_efi_devnum}:''${bootswain_efi_bootpart} /EFI/BOOT; then true; else echo "Unable to list /EFI/BOOT on ''${bootswain_efi_devtype} ''${bootswain_efi_devnum}:''${bootswain_efi_bootpart}"; false; fi; if fatload ''${bootswain_efi_devtype} ''${bootswain_efi_devnum}:''${bootswain_efi_bootpart} ''${kernel_addr_r} ''${bootswain_efi_loader}; then echo "Loaded EFI payload size ''${filesize}"; if test -n "''${fdtcontroladdr}"; then bootefi ''${kernel_addr_r} ''${fdtcontroladdr}; else bootefi ''${kernel_addr_r}; fi; else echo "Failed to load ''${bootswain_efi_loader}"; false; fi'
    setenv bootswain_boot_emmc 'run bootswain_boot_prepare_emmc; echo "Scanning eMMC bootflows"; if bootflow scan -lb ''${bootswain_boot_target_emmc}; then true; else echo "No bootable eMMC bootflow; trying direct EFI"; setenv bootswain_efi_target ''${bootswain_boot_target_emmc}; setenv bootswain_efi_devtype mmc; setenv bootswain_efi_devnum 0; setenv bootswain_efi_bootpart 1; run bootswain_boot_direct_efi; fi'
    setenv bootswain_boot_sd 'run bootswain_boot_prepare_sd; echo "Scanning SD bootflows"; if bootflow scan -lb ''${bootswain_boot_target_sd}; then true; else echo "No bootable SD bootflow; trying direct EFI"; setenv bootswain_efi_target ''${bootswain_boot_target_sd}; setenv bootswain_efi_devtype mmc; setenv bootswain_efi_devnum 1; setenv bootswain_efi_bootpart 1; run bootswain_boot_direct_efi; fi'
    setenv bootswain_boot_usb 'run bootswain_boot_prepare_usb; echo "Scanning USB bootflows"; if bootflow scan -lb ''${bootswain_boot_target_usb}; then true; else echo "No bootable USB bootflow; trying direct EFI"; setenv bootswain_efi_target ''${bootswain_boot_target_usb}; setenv bootswain_efi_devtype usb; setenv bootswain_efi_devnum 0; setenv bootswain_efi_bootpart 1; run bootswain_boot_direct_efi; fi'
    setenv bootswain_boot_nvme 'run bootswain_boot_prepare_nvme; echo "Scanning NVMe bootflows"; if bootflow scan -lb ''${bootswain_boot_target_nvme}; then true; else echo "No bootable NVMe bootflow; trying direct EFI"; setenv bootswain_efi_target ''${bootswain_boot_target_nvme}; setenv bootswain_efi_devtype nvme; setenv bootswain_efi_devnum 0; setenv bootswain_efi_bootpart 1; run bootswain_boot_direct_efi; fi'

    setenv bootswain_check_board 'echo "Checking ROCKPro64 board identity"; if fdt addr ''${fdtcontroladdr}; then if fdt get value bootswain_detected_compatible / compatible; then if test "''${bootswain_detected_compatible}" = "''${bootswain_expected_compatible}"; then true; else if test "''${bootswain_detected_compatible}" = "''${bootswain_expected_compatible_v2_1}"; then true; else echo "Unexpected board compatible: ''${bootswain_detected_compatible}"; false; fi; fi; else echo "Unable to read control FDT compatible"; false; fi; else echo "Unable to access control FDT"; false; fi'
    setenv bootswain_probe_spi 'echo "Probing SPI flash"; if sf probe; then if test -n "''${sf_size}"; then if test "''${sf_size}" = "''${bootswain_spi_size}"; then true; else echo "Unexpected SPI size: ''${sf_size}"; false; fi; else echo "SPI size variable unavailable; continuing with configured 16 MiB bound"; true; fi; else echo "SPI probe failed"; false; fi'
    setenv bootswain_load_spi_payload 'echo "Loading SPI payload from ''${bootswain_spi_source_devtype} ''${bootswain_spi_source_devnum}:''${bootswain_spi_source_bootpart}"; if fatload ''${bootswain_spi_source_devtype} ''${bootswain_spi_source_devnum}:''${bootswain_spi_source_bootpart} ''${kernel_addr_r} ''${bootswain_spi_image}; then true; else echo "Primary SPI payload not found at ''${bootswain_spi_image}; trying ''${bootswain_spi_image_fallback}"; if fatload ''${bootswain_spi_source_devtype} ''${bootswain_spi_source_devnum}:''${bootswain_spi_source_bootpart} ''${kernel_addr_r} ''${bootswain_spi_image_fallback}; then true; else echo "Failed to load SPI payload from ''${bootswain_spi_source_devtype} ''${bootswain_spi_source_devnum}:''${bootswain_spi_source_bootpart}"; false; fi; fi'
    setenv bootswain_check_spi_payload 'echo "Checking SPI payload size ''${filesize}"; if itest ''${filesize} -gt 0; then if itest ''${filesize} -le ''${bootswain_spi_max_payload_size}; then true; else echo "SPI payload is too large for configured flash bound"; false; fi; else echo "SPI payload size is empty"; false; fi'
    setenv bootswain_compare_spi_payload 'echo "Comparing installed SPI firmware"; if sf read ''${bootswain_spi_verify_addr} 0 ''${filesize}; then if cmp.b ''${kernel_addr_r} ''${bootswain_spi_verify_addr} ''${filesize}; then echo "SPI firmware already matches bootswain payload"; true; else echo "SPI firmware differs from bootswain payload"; false; fi; else echo "SPI compare read failed"; false; fi'
    setenv bootswain_write_spi_payload 'echo "Writing SPI payload"; if sf update ''${kernel_addr_r} 0 ''${filesize}; then true; else echo "SPI flash write failed"; false; fi'
    setenv bootswain_verify_spi_payload 'echo "Verifying SPI payload"; if sf read ''${bootswain_spi_verify_addr} 0 ''${filesize}; then if cmp.b ''${kernel_addr_r} ''${bootswain_spi_verify_addr} ''${filesize}; then echo "SPI verify complete"; true; else echo "SPI verify mismatch"; false; fi; else echo "SPI verify read failed"; false; fi'
    setenv bootswain_update_spi_if_needed 'if run bootswain_check_board; then if ''${bootswain_spi_source_devtype} dev ''${bootswain_spi_source_devnum}; then if ''${bootswain_spi_source_devtype} rescan; then if run bootswain_probe_spi; then if run bootswain_load_spi_payload; then if run bootswain_check_spi_payload; then if run bootswain_compare_spi_payload; then echo "Continuing boot with current SPI firmware"; true; else echo "Updating SPI firmware from bootswain NixOS payload"; if run bootswain_write_spi_payload; then if run bootswain_verify_spi_payload; then echo "SPI firmware update complete"; true; else false; fi; else false; fi; fi; else false; fi; else false; fi; else false; fi; else echo "Boot source rescan failed"; false; fi; else echo "Failed to select boot source"; false; fi; else false; fi'

    echo "bootswain ROCKPro64 NixOS boot handoff"
    echo "SPI payload: ''${bootswain_spi_image}"
    echo "SPI payload fallback: ''${bootswain_spi_image_fallback}"
    echo "SPI payload source: ''${bootswain_spi_source_devtype} ''${bootswain_spi_source_devnum}:''${bootswain_spi_source_bootpart}"
    if run bootswain_update_spi_if_needed; then
    	if run bootswain_prepare_os_bootmeths; then run bootswain_boot_auto; else echo "Failed to select OS boot methods"; false; fi
    else
    	echo "bootswain SPI update failed; continuing normal boot scan"
    	if run bootswain_prepare_os_bootmeths; then run bootswain_boot_auto; else echo "Failed to select OS boot methods"; false; fi
    fi
  '';
  rockpro64NixosUpdaterBootScript = mkBootScript {
    name = "rockpro64-nixos-updater-boot.scr.uimg";
    bootCmd = rockpro64NixosUpdaterBootCmd;
  };

  rockpro64InstallerUboot = mkRockpro64Uboot {
    role = "bootswain-installer";
    envFile = "${rockpro64FirmwareDir}/uboot.env";
  };
  rockpro64FirmwareUboot = mkRockpro64Uboot {
    role = "bootswain-firmware";
    envFile = rockpro64FirmwareEnv;
  };

  rockpro64UbootVersion = rockpro64FirmwareUboot.version;
  rockpro64UbootTag = "v${rockpro64UbootVersion}";
  rockpro64ReleaseCandidate = "${rockpro64UbootVersion}-rockpro64-rc.0";
  rockpro64StableRelease = "${rockpro64UbootVersion}-rockpro64-stable.0";
  rockpro64ExperimentalRelease = "${rockpro64UbootVersion}-rockpro64-experimental.0";
  raspberryPi3BPlusReleaseCandidate = "${raspberryPi3BPlusUboot.version}-raspberrypi3bplus-rc.0";
  rockpro64StableValidationRecord = "docs/src/validation/stable-evidence.md";
  stableEvidenceDir = "${validationDir}/rockpro64/stable";
  requiredStableScenarios = [
    "spi-installer-menu"
    "spi-install"
    "spi-reinstall"
    "spi-erase"
    "spi-post-install-prompt"
    "no-bootable-media-ui"
    "recovery-console"
    "sd-uefi-boot"
    "sd-extlinux-boot"
    "emmc-uefi-boot"
    "emmc-extlinux-boot"
    "usb-uefi-boot"
    "usb-extlinux-boot"
  ];
  requiredStableScenariosText = builtins.concatStringsSep "\n" requiredStableScenarios;

  rockpro64Uboot = pkgs.linkFarm "rockpro64-uboot" [
    {
      name = "README.md";
      path = "${rockpro64FirmwareDir}/README.md";
    }
    {
      name = "contract.md";
      path = "${rockpro64FirmwareDir}/contract.md";
    }
    {
      name = "installer.boot.cmd";
      path = rockpro64InstallerBootCmd;
    }
    {
      name = "firmware.boot.cmd";
      path = rockpro64FirmwareBootCmd;
    }
    {
      name = "boot.cmd";
      path = rockpro64InstallerBootCmd;
    }
    {
      name = "boot.fragment.cmd";
      path = "${rockpro64FirmwareDir}/boot.cmd";
    }
    {
      name = "firmware.fragment.cmd";
      path = rockpro64FirmwareFragment;
    }
    {
      name = "generic-boot-menu.cmd";
      path = "${firmwareLibDir}/generic-boot-menu.cmd";
    }
    {
      name = "installer.boot.scr.uimg";
      path = rockpro64InstallerBootScript;
    }
    {
      name = "firmware.boot.scr.uimg";
      path = rockpro64FirmwareBootScript;
    }
    {
      name = "nixos-updater.boot.cmd";
      path = rockpro64NixosUpdaterBootCmd;
    }
    {
      name = "nixos-updater.boot.scr.uimg";
      path = rockpro64NixosUpdaterBootScript;
    }
    {
      name = "boot.scr.uimg";
      path = rockpro64InstallerBootScript;
    }
    {
      name = "uboot.env";
      path = "${rockpro64FirmwareDir}/uboot.env";
    }
    {
      name = "firmware.env";
      path = rockpro64FirmwareEnv;
    }
    {
      name = "u-boot.config.fragment";
      path = "${rockpro64FirmwareDir}/u-boot.config.fragment";
    }
    {
      name = "bl31.elf";
      path = "${rockpro64TfA}/bl31.elf";
    }
    {
      name = "installer-idbloader.img";
      path = "${rockpro64InstallerUboot}/idbloader.img";
    }
    {
      name = "installer-u-boot.itb";
      path = "${rockpro64InstallerUboot}/u-boot.itb";
    }
    {
      name = "installer-u-boot-rockchip.bin";
      path = "${rockpro64InstallerUboot}/u-boot-rockchip.bin";
    }
    {
      name = "firmware-idbloader.img";
      path = "${rockpro64FirmwareUboot}/idbloader.img";
    }
    {
      name = "firmware-u-boot.itb";
      path = "${rockpro64FirmwareUboot}/u-boot.itb";
    }
    {
      name = "firmware-u-boot-rockchip.bin";
      path = "${rockpro64FirmwareUboot}/u-boot-rockchip.bin";
    }
    {
      name = "firmware-u-boot-rockchip-spi.bin";
      path = "${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin";
    }
    {
      name = "idbloader.img";
      path = "${rockpro64FirmwareUboot}/idbloader.img";
    }
    {
      name = "u-boot.itb";
      path = "${rockpro64FirmwareUboot}/u-boot.itb";
    }
    {
      name = "u-boot-rockchip.bin";
      path = "${rockpro64FirmwareUboot}/u-boot-rockchip.bin";
    }
    {
      name = "u-boot-rockchip-spi.bin";
      path = "${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin";
    }
  ];

  rockpro64SpiFirmware = pkgs.linkFarm "rockpro64-spi-firmware" [
    {
      name = "README.md";
      path = "${rockpro64FirmwareDir}/README.md";
    }
    {
      name = "contract.md";
      path = "${rockpro64FirmwareDir}/contract.md";
    }
    {
      name = "spi-layout.md";
      path = "${rockpro64FirmwareDir}/spi-layout.md";
    }
    {
      name = "boot.cmd";
      path = rockpro64FirmwareBootCmd;
    }
    {
      name = "boot.fragment.cmd";
      path = rockpro64FirmwareFragment;
    }
    {
      name = "nixos-updater.boot.cmd";
      path = rockpro64NixosUpdaterBootCmd;
    }
    {
      name = "generic-boot-menu.cmd";
      path = "${firmwareLibDir}/generic-boot-menu.cmd";
    }
    {
      name = "boot.scr.uimg";
      path = rockpro64FirmwareBootScript;
    }
    {
      name = "nixos-updater.boot.scr.uimg";
      path = rockpro64NixosUpdaterBootScript;
    }
    {
      name = "uboot.env";
      path = rockpro64FirmwareEnv;
    }
    {
      name = "u-boot.config.fragment";
      path = "${rockpro64FirmwareDir}/u-boot.config.fragment";
    }
    {
      name = "bl31.elf";
      path = "${rockpro64TfA}/bl31.elf";
    }
    {
      name = "idbloader.img";
      path = "${rockpro64FirmwareUboot}/idbloader.img";
    }
    {
      name = "u-boot.itb";
      path = "${rockpro64FirmwareUboot}/u-boot.itb";
    }
    {
      name = "u-boot-rockchip.bin";
      path = "${rockpro64FirmwareUboot}/u-boot-rockchip.bin";
    }
    {
      name = "u-boot-rockchip-spi.bin";
      path = "${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin";
    }
  ];

  rockpro64SpiInstallerImg =
    pkgs.runCommand "spi.installer.img" {
      nativeBuildInputs = [
        pkgs.dosfstools
        pkgs.mtools
        pkgs.util-linux
      ];
    } ''
          boot_partition="$TMPDIR/bootpart.img"

          truncate -s 64M "$boot_partition"
          mkfs.vfat -n BOOTSWAIN "$boot_partition" >/dev/null
          mmd -i "$boot_partition" ::/boot
          mcopy -i "$boot_partition" ${rockpro64InstallerBootScript} ::/boot.scr.uimg
          mcopy -i "$boot_partition" ${rockpro64InstallerBootScript} ::/boot/boot.scr.uimg
          mcopy -i "$boot_partition" ${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin ::/u-boot-rockchip-spi.bin
          mcopy -i "$boot_partition" ${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin ::/boot/u-boot-rockchip-spi.bin

          truncate -s 96M "$out"
          sfdisk "$out" <<'EOF' >/dev/null
      label: dos
      unit: sectors
      sector-size: 512

      start=32768, size=131072, type=c, bootable
      EOF

          dd if=${rockpro64InstallerUboot}/u-boot-rockchip.bin of="$out" bs=512 seek=64 conv=notrunc status=none
          dd if="$boot_partition" of="$out" bs=512 seek=32768 conv=notrunc status=none
    '';

  rockpro64SpiInstallerImgExperimental = pkgs.runCommand "spi.installer.experimental.img" {} ''
    cp ${rockpro64SpiInstallerImg} "$out"
  '';

  rockpro64SharedDiskImageImg =
    pkgs.runCommand "shared.disk-image.img" {
      nativeBuildInputs = [
        pkgs.dosfstools
        pkgs.mtools
        pkgs.util-linux
      ];
    } ''
          firmware_partition="$TMPDIR/firmwarepart.img"
          metadata="$TMPDIR/bootswain-release.txt"

          truncate -s 64M "$firmware_partition"
          mkfs.vfat -n BSWNFW "$firmware_partition" >/dev/null
          mmd -i "$firmware_partition" ::/boot
          mcopy -i "$firmware_partition" ${rockpro64FirmwareBootScript} ::/boot.scr.uimg
          mcopy -i "$firmware_partition" ${rockpro64FirmwareBootScript} ::/boot/boot.scr.uimg
          cat > "$metadata" <<EOF
      bootswain ROCKPro64 shared-storage firmware image
      release=${rockpro64ReleaseCandidate}
      channel=release-candidate
      hardware-validation=required-before-stable-claim
      EOF
          mcopy -i "$firmware_partition" "$metadata" ::/bootswain-release.txt

          truncate -s 96M "$out"
          sfdisk "$out" <<'EOF' >/dev/null
      label: dos
      unit: sectors
      sector-size: 512

      start=32768, size=131072, type=c, bootable
      EOF

          dd if=${rockpro64FirmwareUboot}/u-boot-rockchip.bin of="$out" bs=512 seek=64 conv=notrunc status=none
          dd if="$firmware_partition" of="$out" bs=512 seek=32768 conv=notrunc status=none
    '';

  mkRockpro64ReleaseBundle = {
    name,
    release,
    channel,
    installerImage,
    spiInstallerFlashable,
    sharedDiskFlashable,
    validationPerformed,
    validationRecordJson,
    validationClaimsJson,
    unsupportedPathsJson,
    validationNote,
    requireEvidence ? false,
  }:
    pkgs.runCommand name {
      nativeBuildInputs = [pkgs.jq];
    } ''
            mkdir -p "$out"

            cp ${installerImage} "$out/spi.installer.img"
            cp ${rockpro64SharedDiskImageImg} "$out/shared.disk-image.img"
            cp ${rockpro64TfA}/bl31.elf "$out/bl31.elf"
            cp ${rockpro64FirmwareUboot}/idbloader.img "$out/idbloader.img"
            cp ${rockpro64FirmwareUboot}/u-boot.itb "$out/u-boot.itb"
            cp ${rockpro64FirmwareUboot}/u-boot-rockchip.bin "$out/u-boot-rockchip.bin"
            cp ${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin "$out/u-boot-rockchip-spi.bin"

            spi_installer_size=$(stat -c%s "$out/spi.installer.img")
            spi_installer_sha=$(sha256sum "$out/spi.installer.img" | cut -d' ' -f1)
            shared_disk_size=$(stat -c%s "$out/shared.disk-image.img")
            shared_disk_sha=$(sha256sum "$out/shared.disk-image.img" | cut -d' ' -f1)
            idbloader_size=$(stat -c%s "$out/idbloader.img")
            idbloader_sha=$(sha256sum "$out/idbloader.img" | cut -d' ' -f1)
            uboot_itb_size=$(stat -c%s "$out/u-boot.itb")
            uboot_itb_sha=$(sha256sum "$out/u-boot.itb" | cut -d' ' -f1)
            uboot_rockchip_size=$(stat -c%s "$out/u-boot-rockchip.bin")
            uboot_rockchip_sha=$(sha256sum "$out/u-boot-rockchip.bin" | cut -d' ' -f1)
            uboot_rockchip_spi_size=$(stat -c%s "$out/u-boot-rockchip-spi.bin")
            uboot_rockchip_spi_sha=$(sha256sum "$out/u-boot-rockchip-spi.bin" | cut -d' ' -f1)

            cat > "$out/release.json" <<EOF
      {
        "schema_version": 1,
        "release": "${release}",
        "board": "rock-pro64",
        "sources": {
          "u_boot": "${rockpro64UbootTag}",
          "trusted_firmware_a": "v2.14.0",
          "bootswain": "${bootswainRevision}",
          "nixpkgs": "${nixpkgsRevision}"
        },
        "storage_layout": {
          "spi_size_bytes": 16777216,
          "spi_firmware_offset_bytes": 0,
          "spi_firmware_size_bytes": ${"$"}uboot_rockchip_spi_size,
          "environment_offset_bytes": null,
          "environment_size_bytes": null,
          "shared_storage_firmware_partition": "BSWNFW"
        },
        "boot_policy": {
          "default_order": ["emmc", "sd", "usb", "nvme"],
          "one_shot_usb_order": ["usb", "emmc", "sd", "nvme"],
          "one_shot_usb_marker": "/boot/bootswain/next-boot-usb",
          "preferred_protocols": ["uefi", "extlinux"],
          "no_bootable_media_behavior": "show boot menu, diagnostics, and U-Boot shell over 115200 serial"
        },
        "environment_policy": {
          "persistent": false,
          "location": null,
          "stale_environment_safe": true,
          "notes": "ROCKPro64 images use a stateless environment policy; no persistent SPI environment offset is claimed for this release."
        },
        "artifacts": [
          {
            "kind": "spi-installer",
            "path": "spi.installer.img",
            "compression": "none",
            "size_bytes": ${"$"}spi_installer_size,
            "sha256": "${"$"}spi_installer_sha",
            "required_device_size_bytes": ${"$"}spi_installer_size,
            "flashable": ${
        if spiInstallerFlashable
        then "true"
        else "false"
      }
          },
          {
            "kind": "shared-disk-image",
            "path": "shared.disk-image.img",
            "compression": "none",
            "size_bytes": ${"$"}shared_disk_size,
            "sha256": "${"$"}shared_disk_sha",
            "required_device_size_bytes": ${"$"}shared_disk_size,
            "flashable": ${
        if sharedDiskFlashable
        then "true"
        else "false"
      }
          },
          {
            "kind": "idbloader",
            "path": "idbloader.img",
            "compression": "none",
            "size_bytes": ${"$"}idbloader_size,
            "sha256": "${"$"}idbloader_sha",
            "required_device_size_bytes": null,
            "flashable": false
          },
          {
            "kind": "u-boot-itb",
            "path": "u-boot.itb",
            "compression": "none",
            "size_bytes": ${"$"}uboot_itb_size,
            "sha256": "${"$"}uboot_itb_sha",
            "required_device_size_bytes": null,
            "flashable": false
          },
          {
            "kind": "u-boot-rockchip",
            "path": "u-boot-rockchip.bin",
            "compression": "none",
            "size_bytes": ${"$"}uboot_rockchip_size,
            "sha256": "${"$"}uboot_rockchip_sha",
            "required_device_size_bytes": null,
            "flashable": false
          },
          {
            "kind": "u-boot-rockchip-spi",
            "path": "u-boot-rockchip-spi.bin",
            "compression": "none",
            "size_bytes": ${"$"}uboot_rockchip_spi_size,
            "sha256": "${"$"}uboot_rockchip_spi_sha",
            "required_device_size_bytes": null,
            "flashable": false
          }
        ],
        "validation_claims": ${validationClaimsJson},
        "unsupported_paths": ${unsupportedPathsJson}
      }
      EOF

            cp "$out/release.json" "$out/manifest.json"
            cat > "$out/provenance.json" <<EOF
      {
        "board": "pine64-rockpro64",
        "soc": "rockchip-rk3399",
        "release": "${release}",
        "channel": "${channel}",
        "hardware_validation": {
          "performed": ${
        if validationPerformed
        then "true"
        else "false"
      },
          "record": ${validationRecordJson},
          "note": "${validationNote}"
        },
        "source_revisions": {
          "u_boot": "${rockpro64UbootTag}",
          "trusted_firmware_a": "v2.14.0",
          "nixpkgs": "${nixpkgsRevision}",
          "bootswain": "${bootswainRevision}"
        },
        "source_files": {
          "README": "${rockpro64FirmwareDir}/README.md",
          "contract": "${rockpro64FirmwareDir}/contract.md",
          "spi_layout": "${rockpro64FirmwareDir}/spi-layout.md",
          "boot_cmd_template": "${firmwareLibDir}/generic-boot-menu.cmd",
          "installer_boot_cmd_board_fragment": "${rockpro64FirmwareDir}/boot.cmd",
          "firmware_boot_cmd_board_fragment": "${rockpro64FirmwareFragment}",
          "installer_boot_cmd": "${rockpro64InstallerBootCmd}",
          "firmware_boot_cmd": "${rockpro64FirmwareBootCmd}",
          "installer_uboot_env": "${rockpro64FirmwareDir}/uboot.env",
          "firmware_uboot_env": "${rockpro64FirmwareEnv}",
          "config_fragment": "${rockpro64FirmwareDir}/u-boot.config.fragment"
        },
        "built_artifacts": {
          "installer_boot_script": "${rockpro64InstallerBootScript}",
          "firmware_boot_script": "${rockpro64FirmwareBootScript}",
          "tf_a_bl31": "${rockpro64TfA}/bl31.elf",
          "installer_u_boot_rockchip": "${rockpro64InstallerUboot}/u-boot-rockchip.bin",
          "firmware_idbloader": "${rockpro64FirmwareUboot}/idbloader.img",
          "firmware_u_boot_itb": "${rockpro64FirmwareUboot}/u-boot.itb",
          "firmware_u_boot_rockchip": "${rockpro64FirmwareUboot}/u-boot-rockchip.bin",
          "firmware_u_boot_rockchip_spi": "${rockpro64FirmwareUboot}/u-boot-rockchip-spi.bin",
          "spi_installer_image": "${installerImage}",
          "shared_disk_image": "${rockpro64SharedDiskImageImg}"
        }
      }
      EOF

            ${
        if requireEvidence
        then ''
          if [ ! -s ${stableEvidenceDir}/validation-run.json ]; then
            echo "missing stable hardware validation evidence: validation/rockpro64/stable/validation-run.json" >&2
            exit 1
          fi
          mkdir -p "$out/validation"
          cp -r ${stableEvidenceDir}/. "$out/validation/"
          required_scenarios=${pkgs.writeText "rockpro64-required-stable-scenarios.txt" requiredStableScenariosText}
          while IFS= read -r scenario; do
            [ -n "$scenario" ] || continue
            jq -e --arg scenario "$scenario" '
              any(.outcomes[]; .scenario == $scenario and .status == "passed")
            ' "$out/validation/validation-run.json" >/dev/null || {
              echo "missing passing stable scenario: $scenario" >&2
              exit 1
            }
            jq -r --arg scenario "$scenario" '
              .outcomes[]
              | select(.scenario == $scenario)
              | .logs[]
            ' "$out/validation/validation-run.json" | while IFS= read -r log; do
              [ -n "$log" ] || continue
              if [ ! -s "$out/validation/$log" ]; then
                echo "missing serial log for $scenario: validation/$log" >&2
                exit 1
              fi
            done
          done < "$required_scenarios"
        ''
        else ''
          mkdir -p "$out/validation"
          printf '%s\n' \
            "Stable ROCKPro64 validation evidence has not been imported." \
            "Use validation/rockpro64/stable/validation-run.json plus serial logs before stable promotion." \
            > "$out/validation/README.txt"
        ''
      }

            cat > "$out/release-notes.md" <<EOF
      # ROCKPro64 ${release}

      Channel: ${channel}

      ## Artifacts

      - \`spi.installer.img\`: SD-bootable SPI installer
      - \`shared.disk-image.img\`: shared-storage SD/eMMC firmware image
      - \`idbloader.img\`, \`u-boot.itb\`, \`u-boot-rockchip.bin\`, and \`u-boot-rockchip-spi.bin\`: RK3399 firmware components
      - \`release.json\`: machine-readable release metadata
      - \`sha256sums.txt\`: checksums for published artifacts
      - \`provenance.json\`: source and build provenance

      ## Validation

      Hardware validation performed: ${
        if validationPerformed
        then "yes"
        else "no"
      }

      ${validationNote}

      ## Unsupported

      - phone/tablet volume-button shortcuts
      - USB mass-storage gadget mode
      - network boot
      - custom persistent environment edits
      - NVMe OS boot without release validation logs
      EOF

            (
              cd "$out"
              sha256sum \
                spi.installer.img \
                shared.disk-image.img \
                bl31.elf \
                idbloader.img \
                u-boot.itb \
                u-boot-rockchip.bin \
                u-boot-rockchip-spi.bin \
                release.json \
                release-notes.md \
                provenance.json \
                > sha256sums.txt
            )
    '';

  releaseCandidateClaimsJson = ''
    [
      {
        "target": "spi",
        "protocols": ["u-boot-shell"],
        "tested": false,
        "scenarios": [
          "spi-installer-menu",
          "spi-install",
          "spi-reinstall",
          "spi-erase",
          "spi-post-install-prompt",
          "no-bootable-media-ui",
          "recovery-console"
        ],
        "notes": "Release-candidate only; stable SPI claims require imported ROCKPro64 serial validation evidence."
      },
      {
        "target": "sd",
        "protocols": ["uefi", "extlinux"],
        "tested": false,
        "scenarios": ["sd-uefi-boot", "sd-extlinux-boot"],
        "notes": "Shared-storage SD support is built but not stable until hardware logs are imported."
      },
      {
        "target": "emmc",
        "protocols": ["uefi", "extlinux"],
        "tested": false,
        "scenarios": ["emmc-uefi-boot", "emmc-extlinux-boot"],
        "notes": "Shared-storage eMMC support is built but not stable until hardware logs are imported."
      },
      {
        "target": "usb",
        "protocols": ["uefi", "extlinux"],
        "tested": false,
        "scenarios": ["usb-uefi-boot", "usb-extlinux-boot"],
        "notes": "USB boot support is built but not stable until hardware logs are imported."
      }
    ]
  '';

  stableClaimsJson = ''
    [
      {
        "target": "spi",
        "protocols": ["u-boot-shell"],
        "tested": true,
        "scenarios": [
          "spi-installer-menu",
          "spi-install",
          "spi-reinstall",
          "spi-erase",
          "spi-post-install-prompt",
          "no-bootable-media-ui",
          "recovery-console"
        ],
        "notes": "Accepted only when the bundled validation evidence passes."
      },
      {
        "target": "sd",
        "protocols": ["uefi", "extlinux"],
        "tested": true,
        "scenarios": ["sd-uefi-boot", "sd-extlinux-boot"],
        "notes": "Accepted only when the bundled validation evidence passes."
      },
      {
        "target": "emmc",
        "protocols": ["uefi", "extlinux"],
        "tested": true,
        "scenarios": ["emmc-uefi-boot", "emmc-extlinux-boot"],
        "notes": "Accepted only when the bundled validation evidence passes."
      },
      {
        "target": "usb",
        "protocols": ["uefi", "extlinux"],
        "tested": true,
        "scenarios": ["usb-uefi-boot", "usb-extlinux-boot"],
        "notes": "Accepted only when the bundled validation evidence passes."
      }
    ]
  '';

  unsupportedPathsJson = ''
    [
      "phone/tablet volume-button shortcuts",
      "USB mass-storage gadget mode",
      "network boot",
      "custom persistent environment edits",
      "NVMe OS boot paths without release validation logs",
      "advertising unvalidated storage classes as stable support"
    ]
  '';

  rockpro64ReleaseBundle = mkRockpro64ReleaseBundle {
    name = "rockpro64-release-candidate-bundle";
    release = rockpro64ReleaseCandidate;
    channel = "release-candidate";
    installerImage = rockpro64SpiInstallerImg;
    spiInstallerFlashable = true;
    sharedDiskFlashable = false;
    validationPerformed = false;
    validationRecordJson = "\"${rockpro64StableValidationRecord}\"";
    validationClaimsJson = releaseCandidateClaimsJson;
    unsupportedPathsJson = unsupportedPathsJson;
    validationNote = "Release candidate only; stable promotion requires validation/rockpro64/stable evidence.";
  };

  rockpro64StableReleaseBundle = mkRockpro64ReleaseBundle {
    name = "rockpro64-stable-release-bundle";
    release = rockpro64StableRelease;
    channel = "stable";
    installerImage = rockpro64SpiInstallerImg;
    spiInstallerFlashable = true;
    sharedDiskFlashable = true;
    validationPerformed = true;
    validationRecordJson = "\"validation/\"";
    validationClaimsJson = stableClaimsJson;
    unsupportedPathsJson = unsupportedPathsJson;
    validationNote = "Stable Full Phase 0 claims require bundled ROCKPro64 validation evidence.";
    requireEvidence = true;
  };

  rockpro64ReleaseBundleExperimental = mkRockpro64ReleaseBundle {
    name = "rockpro64-release-bundle-experimental";
    release = rockpro64ExperimentalRelease;
    channel = "experimental";
    installerImage = rockpro64SpiInstallerImgExperimental;
    spiInstallerFlashable = true;
    sharedDiskFlashable = false;
    validationPerformed = false;
    validationRecordJson = "null";
    validationClaimsJson = ''
      [
        {
          "target": "spi",
          "protocols": ["u-boot-shell"],
          "tested": false,
          "scenarios": ["spi-install", "spi-erase"],
          "notes": "Experimental channel is flashable for developer lab use and does not expand stable support claims."
        }
      ]
    '';
    unsupportedPathsJson = unsupportedPathsJson;
    validationNote = "Experimental channel is flashable for developer lab use, but it is not a stable hardware support claim.";
  };

  raspberryPi3BPlusConfigTxt = pkgs.writeText "raspberrypi3bplus-config.txt" ''
    [pi3]
    kernel=u-boot-rpi3.bin

    # Otherwise the serial output will be garbled.
    core_freq=250

    [all]
    # Boot in 64-bit mode.
    arm_64bit=1

    # U-Boot needs this to work, regardless of whether UART is actually used or not.
    enable_uart=1

    # Prevent the firmware from smashing the framebuffer setup done by the mainline kernel
    # when attempting to show low-voltage or overtemperature warnings.
    avoid_warnings=1
  '';

  raspberryPi3BPlusFirmwarePackage = pkgs.linkFarm "raspberrypi3bplus-firmware" [
    {
      name = "bootcode.bin";
      path = "${raspberryPi3BPlusFirmwareBoot}/bootcode.bin";
    }
    {
      name = "start.elf";
      path = "${raspberryPi3BPlusFirmwareBoot}/start.elf";
    }
    {
      name = "start_cd.elf";
      path = "${raspberryPi3BPlusFirmwareBoot}/start_cd.elf";
    }
    {
      name = "start_db.elf";
      path = "${raspberryPi3BPlusFirmwareBoot}/start_db.elf";
    }
    {
      name = "start_x.elf";
      path = "${raspberryPi3BPlusFirmwareBoot}/start_x.elf";
    }
    {
      name = "fixup.dat";
      path = "${raspberryPi3BPlusFirmwareBoot}/fixup.dat";
    }
    {
      name = "fixup_cd.dat";
      path = "${raspberryPi3BPlusFirmwareBoot}/fixup_cd.dat";
    }
    {
      name = "fixup_db.dat";
      path = "${raspberryPi3BPlusFirmwareBoot}/fixup_db.dat";
    }
    {
      name = "fixup_x.dat";
      path = "${raspberryPi3BPlusFirmwareBoot}/fixup_x.dat";
    }
    {
      name = "bcm2710-rpi-3-b-plus.dtb";
      path = "${raspberryPi3BPlusFirmwareBoot}/bcm2710-rpi-3-b-plus.dtb";
    }
    {
      name = "u-boot-rpi3.bin";
      path = "${raspberryPi3BPlusUboot}/u-boot.bin";
    }
    {
      name = "config.txt";
      path = raspberryPi3BPlusConfigTxt;
    }
    {
      name = "LICENCE.broadcom";
      path = "${raspberryPi3BPlusFirmwareBoot}/LICENCE.broadcom";
    }
    {
      name = "COPYING.linux";
      path = "${raspberryPi3BPlusFirmwareBoot}/COPYING.linux";
    }
  ];

  raspberryPi3BPlusBootPartitionImage = pkgs.runCommand "raspberrypi3bplus.boot-partition.img" {
    nativeBuildInputs = [
      pkgs.dosfstools
      pkgs.mtools
    ];
  } ''
    truncate -s 64M "$out"
    mkfs.vfat -n RPI3BOOT "$out" >/dev/null
    for file in ${raspberryPi3BPlusFirmwarePackage}/*; do
      mcopy -i "$out" "$file" "::/$(basename "$file")"
    done
  '';

  raspberryPi3BPlusReleaseBundle = pkgs.runCommand "raspberrypi3bplus-release-candidate-bundle" {
    nativeBuildInputs = [pkgs.jq];
  } ''
    mkdir -p "$out/firmware"

    cp ${raspberryPi3BPlusBootPartitionImage} "$out/boot-partition.img"
    cp ${raspberryPi3BPlusFirmwarePackage}/u-boot-rpi3.bin "$out/u-boot-rpi3.bin"
    cp -r ${raspberryPi3BPlusFirmwarePackage}/. "$out/firmware/"

    boot_partition_size=$(stat -c%s "$out/boot-partition.img")
    boot_partition_sha=$(sha256sum "$out/boot-partition.img" | cut -d' ' -f1)
    uboot_size=$(stat -c%s "$out/u-boot-rpi3.bin")
    uboot_sha=$(sha256sum "$out/u-boot-rpi3.bin" | cut -d' ' -f1)
    firmware_tar="$out/raspberry-pi-firmware.tar"
    tar -C "$out/firmware" -cf "$firmware_tar" .
    firmware_size=$(stat -c%s "$firmware_tar")
    firmware_sha=$(sha256sum "$firmware_tar" | cut -d' ' -f1)

    cat > "$out/release.json" <<EOF
{
  "schema_version": 1,
  "release": "${raspberryPi3BPlusReleaseCandidate}",
  "board": "raspberry-pi-3-b-plus",
  "sources": {
    "u_boot": "v${raspberryPi3BPlusUboot.version}",
    "trusted_firmware_a": null,
    "bootswain": "${bootswainRevision}",
    "nixpkgs": "${nixpkgsRevision}"
  },
  "storage_layout": {
    "spi_size_bytes": null,
    "spi_firmware_offset_bytes": null,
    "spi_firmware_size_bytes": null,
    "environment_offset_bytes": null,
    "environment_size_bytes": null,
    "shared_storage_firmware_partition": "RPI3BOOT"
  },
  "boot_policy": {
    "default_order": ["sd", "usb"],
    "preferred_protocols": ["extlinux"],
    "no_bootable_media_behavior": "Raspberry Pi firmware loads U-Boot from FAT boot media; U-Boot then uses extlinux."
  },
  "environment_policy": {
    "persistent": false,
    "location": null,
    "stale_environment_safe": true,
    "notes": "Raspberry Pi 3 B+ release-candidate support uses generated boot firmware files and does not claim a persistent U-Boot environment."
  },
  "artifacts": [
    {
      "kind": "boot-partition-image",
      "path": "boot-partition.img",
      "compression": "none",
      "size_bytes": $boot_partition_size,
      "sha256": "$boot_partition_sha",
      "required_device_size_bytes": $boot_partition_size,
      "flashable": true
    },
    {
      "kind": "raspberry-pi-firmware",
      "path": "raspberry-pi-firmware.tar",
      "compression": "none",
      "size_bytes": $firmware_size,
      "sha256": "$firmware_sha",
      "required_device_size_bytes": null,
      "flashable": false
    },
    {
      "kind": "u-boot-rpi3",
      "path": "u-boot-rpi3.bin",
      "compression": "none",
      "size_bytes": $uboot_size,
      "sha256": "$uboot_sha",
      "required_device_size_bytes": null,
      "flashable": false
    }
  ],
  "validation_claims": [
    {
      "target": "sd",
      "protocols": ["extlinux"],
      "tested": false,
      "scenarios": ["sd-extlinux-boot", "serial-u-boot-prompt"],
      "notes": "Release-candidate only; stable Raspberry Pi 3 B+ claims require imported hardware validation evidence."
    },
    {
      "target": "usb",
      "protocols": ["extlinux"],
      "tested": false,
      "scenarios": ["usb-extlinux-boot"],
      "notes": "USB boot is board-firmware dependent and remains unvalidated for stable support."
    }
  ],
  "unsupported_paths": [
    "stable Raspberry Pi 3 B+ support without hardware validation logs",
    "direct Linux kernel firmware boot",
    "32-bit Raspberry Pi boot flow",
    "EEPROM-style update workflows",
    "network boot"
  ]
}
EOF

    cp "$out/release.json" "$out/manifest.json"
    cat > "$out/provenance.json" <<EOF
{
  "board": "raspberry-pi-3-b-plus",
  "soc": "broadcom-bcm2837b0",
  "release": "${raspberryPi3BPlusReleaseCandidate}",
  "channel": "release-candidate",
  "hardware_validation": {
    "performed": false,
    "record": "validation/raspberrypi3bplus/stable",
    "note": "Release candidate only; stable promotion requires Raspberry Pi 3 B+ hardware validation evidence."
  },
  "source_revisions": {
    "u_boot": "v${raspberryPi3BPlusUboot.version}",
    "raspberrypi_firmware": "${raspberryPi3BPlusFirmware.version}",
    "nixpkgs": "${nixpkgsRevision}",
    "bootswain": "${bootswainRevision}"
  },
  "built_artifacts": {
    "boot_partition_image": "${raspberryPi3BPlusBootPartitionImage}",
    "firmware_package": "${raspberryPi3BPlusFirmwarePackage}",
    "u_boot_rpi3": "${raspberryPi3BPlusUboot}/u-boot.bin",
    "config_txt": "${raspberryPi3BPlusConfigTxt}"
  }
}
EOF

    mkdir -p "$out/validation"
    printf '%s\n' \
      "Stable Raspberry Pi 3 B+ validation evidence has not been imported." \
      "Use validation/raspberrypi3bplus/stable/validation-run.json plus serial logs before stable promotion." \
      > "$out/validation/README.txt"

    cat > "$out/release-notes.md" <<EOF
# Raspberry Pi 3 B+ ${raspberryPi3BPlusReleaseCandidate}

Channel: release-candidate

## Artifacts

- \`boot-partition.img\`: FAT boot partition image with Raspberry Pi firmware, config.txt, bcm2710-rpi-3-b-plus.dtb, and u-boot-rpi3.bin
- \`firmware/\`: unpacked boot firmware payload
- \`raspberry-pi-firmware.tar\`: archived boot firmware payload
- \`u-boot-rpi3.bin\`: U-Boot payload selected by config.txt
- \`release.json\`: machine-readable release metadata
- \`sha256sums.txt\`: checksums for published artifacts
- \`provenance.json\`: source and build provenance

## Validation

Hardware validation performed: no

Release candidate only; stable promotion requires validation/raspberrypi3bplus/stable evidence.
EOF

    (
      cd "$out"
      sha256sum \
        boot-partition.img \
        raspberry-pi-firmware.tar \
        u-boot-rpi3.bin \
        release.json \
        release-notes.md \
        provenance.json \
        > sha256sums.txt
    )
  '';

  raspberryPi3BPlusReleaseManifest = pkgs.runCommand "raspberrypi3bplus-release.json" {} ''
    cp ${raspberryPi3BPlusReleaseBundle}/release.json "$out"
  '';

  raspberryPi3BPlusPackagingCheck = pkgs.runCommand "raspberrypi3bplus-packaging-check" {} ''
    test -s ${raspberryPi3BPlusFirmwarePackage}/bootcode.bin
    test -s ${raspberryPi3BPlusFirmwarePackage}/start.elf
    test -s ${raspberryPi3BPlusFirmwarePackage}/fixup.dat
    test -s ${raspberryPi3BPlusFirmwarePackage}/bcm2710-rpi-3-b-plus.dtb
    test -s ${raspberryPi3BPlusFirmwarePackage}/u-boot-rpi3.bin
    test -s ${raspberryPi3BPlusFirmwarePackage}/config.txt
    grep -q 'kernel=u-boot-rpi3.bin' ${raspberryPi3BPlusFirmwarePackage}/config.txt
    grep -q 'arm_64bit=1' ${raspberryPi3BPlusFirmwarePackage}/config.txt
    test -s ${raspberryPi3BPlusBootPartitionImage}
    test -s ${raspberryPi3BPlusReleaseBundle}/release.json
    test -s ${raspberryPi3BPlusReleaseBundle}/sha256sums.txt
    test -s ${raspberryPi3BPlusReleaseBundle}/boot-partition.img
    test -s ${raspberryPi3BPlusReleaseBundle}/u-boot-rpi3.bin
    test -s ${raspberryPi3BPlusReleaseBundle}/firmware/config.txt
    grep -q '"board": "raspberry-pi-3-b-plus"' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"trusted_firmware_a": null' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"spi_size_bytes": null' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"kind": "boot-partition-image"' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"kind": "raspberry-pi-firmware"' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"kind": "u-boot-rpi3"' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"flashable": true' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q '"tested": false' ${raspberryPi3BPlusReleaseBundle}/release.json
    grep -q 'boot-partition.img' ${raspberryPi3BPlusReleaseBundle}/sha256sums.txt
    grep -q 'Hardware validation performed: no' ${raspberryPi3BPlusReleaseBundle}/release-notes.md
    mkdir -p "$out"
    printf '%s\n' "raspberry pi 3 b+ release-candidate packaging is wired up" > "$out/result"
  '';

  rockpro64ReleaseManifest = pkgs.runCommand "rockpro64-release.json" {} ''
    cp ${rockpro64ReleaseBundle}/release.json "$out"
  '';

  rockpro64ChecksumsProvenance = pkgs.runCommand "rockpro64-checksums-provenance" {} ''
    mkdir -p "$out"
    cp ${rockpro64ReleaseBundle}/sha256sums.txt "$out/sha256sums.txt"
    cp ${rockpro64ReleaseBundle}/provenance.json "$out/provenance.json"
  '';

  rockpro64PackagingCheck = pkgs.runCommand "rockpro64-packaging-check" {} ''
    test -e ${rockpro64Uboot}/README.md
    test -e ${rockpro64SpiFirmware}/spi-layout.md
    test -e ${rockpro64Uboot}/generic-boot-menu.cmd
    test -e ${rockpro64Uboot}/boot.fragment.cmd
    test -e ${rockpro64Uboot}/firmware.fragment.cmd
    test -e ${rockpro64Uboot}/installer-u-boot-rockchip.bin
    test -e ${rockpro64Uboot}/firmware-u-boot-rockchip.bin
    test -e ${rockpro64Uboot}/firmware-u-boot-rockchip-spi.bin
    test -e ${rockpro64SpiFirmware}/boot.scr.uimg
    test -s ${rockpro64ReleaseBundle}/release.json
    test -s ${rockpro64ReleaseBundle}/sha256sums.txt
    test -s ${rockpro64ReleaseBundleExperimental}/release.json
    test -s ${rockpro64ReleaseBundle}/spi.installer.img
    test -s ${rockpro64ReleaseBundle}/shared.disk-image.img
    grep -q '"release": "${rockpro64ReleaseCandidate}"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"release": "${rockpro64ExperimentalRelease}"' ${rockpro64ReleaseBundleExperimental}/release.json
    grep -q '"release": "${rockpro64ReleaseCandidate}"' ${rockpro64ReleaseBundle}/provenance.json
    grep -q '"path": "spi.installer.img"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"path": "shared.disk-image.img"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"path": "u-boot-rockchip-spi.bin"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"nixpkgs": "${nixpkgsRevision}"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"bootswain": "${bootswainRevision}"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"flashable": true' ${rockpro64ReleaseBundle}/release.json
    grep -q '"flashable": false' ${rockpro64ReleaseBundle}/release.json
    grep -q '"target": "spi"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"target": "sd"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"target": "emmc"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"target": "usb"' ${rockpro64ReleaseBundle}/release.json
    grep -q '"tested": false' ${rockpro64ReleaseBundle}/release.json
    grep -q '"scenarios":' ${rockpro64ReleaseBundle}/release.json
    grep -q 'custom persistent environment edits' ${rockpro64ReleaseBundle}/release.json
    grep -q '"performed": false' ${rockpro64ReleaseBundle}/provenance.json
    grep -q 'Full Phase 0 claims require imported hardware validation evidence' ${rockpro64Uboot}/uboot.env
    grep -q '^bootswain_runtime_role=installer$' ${rockpro64Uboot}/uboot.env
    grep -q '^bootswain_runtime_role=firmware$' ${rockpro64Uboot}/firmware.env
    grep -q 'Checking ROCKPro64 board identity' ${rockpro64Uboot}/boot.cmd
    grep -q 'SPI verify complete' ${rockpro64Uboot}/boot.cmd
    grep -q 'SPI erase command failed' ${rockpro64Uboot}/boot.cmd
    grep -q 'bootswain_has_bootmenu' ${rockpro64Uboot}/uboot.env
    grep -q 'if test "''${bootswain_has_bootmenu}" = "1"; then bootmenu ''${bootswain_bootmenu_delay}; else run bootswain_menu_fallback; fi' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootmenu command unavailable; serial command mode active' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootswain serial command mode ready' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'Run: run bootswain_flash_spi' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'SPI payload source:' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootswain_chainloader_role' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootswain_chainloader_has_bootmenu' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -A2 'elif test "''${bootswain_chainloader_role}" = "firmware"; then' ${rockpro64Uboot}/generic-boot-menu.cmd | grep -q 'run bootswain_menu'
    grep -A2 'if test "''${bootswain_chainloader_has_bootmenu}" = "1"; then' ${rockpro64Uboot}/generic-boot-menu.cmd | grep -q 'run bootswain_menu'
    grep -q 'bootswain validation shell ready' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootswain_script_mode' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootswain_refuse_installed_spi_action' ${rockpro64Uboot}/boot.cmd
    grep -q 'SPI flashing is refused from installed SPI firmware' ${rockpro64Uboot}/boot.cmd
    grep -q 'bootswain_auto_upgrade_spi' ${rockpro64Uboot}/boot.cmd
    grep -q 'Auto-upgrading SPI firmware from legacy chainloader' ${rockpro64Uboot}/boot.cmd
    grep -q 'SPI install complete; rebooting' ${rockpro64Uboot}/boot.cmd
    grep -q 'reset' ${rockpro64Uboot}/boot.cmd
    grep -q 'if test "''${bootswain_has_bootmenu}" = "1"; then' ${rockpro64Uboot}/boot.cmd
    grep -q 'run bootswain_auto_upgrade_spi' ${rockpro64Uboot}/boot.cmd
    ! grep -q 'Auto-installing SPI firmware' ${rockpro64Uboot}/boot.cmd
    grep -q 'bootswain_chainload_devtype' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'bootswain_spi_source_devtype' ${rockpro64Uboot}/boot.cmd
    grep -q 'fatload ''${bootswain_spi_source_devtype} ''${bootswain_spi_source_devnum}:''${bootswain_spi_source_bootpart}' ${rockpro64Uboot}/boot.cmd
    ! grep -q 'fatload mmc 0:1' ${rockpro64Uboot}/boot.cmd
    grep -q 'SPI flashing is available only from the installer image' ${rockpro64Uboot}/firmware.boot.cmd
    grep -q 'Comparing installed SPI firmware' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q 'SPI firmware already matches bootswain payload' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q 'Updating SPI firmware from bootswain NixOS payload' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q 'Continuing boot with current SPI firmware' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q 'run bootswain_boot_auto' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q '/bootswain/next-boot-usb' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q 'fatrm ''${bootswain_next_boot_usb_devtype} ''${bootswain_next_boot_usb_devnum}:''${bootswain_next_boot_usb_bootpart} ''${bootswain_next_boot_usb_marker}' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -q 'bootswain_boot_usb_oneshot_order.*bootswain_boot_usb.*bootswain_boot_emmc.*bootswain_boot_sd.*bootswain_boot_nvme' ${rockpro64Uboot}/nixos-updater.boot.cmd
    grep -a -q 'Comparing installed SPI firmware' ${rockpro64Uboot}/nixos-updater.boot.scr.uimg
    grep -q 'Continue boot=' ${rockpro64Uboot}/boot.cmd
    grep -q 'Rescan detected boot options' ${rockpro64Uboot}/generic-boot-menu.cmd
    grep -q 'Boot from eMMC=' ${rockpro64Uboot}/boot.cmd
    grep -q 'Boot from SD=' ${rockpro64Uboot}/boot.cmd
    grep -q 'Boot from USB=' ${rockpro64Uboot}/boot.cmd
    grep -q 'Boot from NVMe=' ${rockpro64Uboot}/boot.cmd
    grep -q 'CONFIG_USE_PREBOOT=y' ${rockpro64Uboot}/u-boot.config.fragment
    grep -q 'CONFIG_AUTOBOOT_USE_MENUKEY=y' ${rockpro64Uboot}/u-boot.config.fragment
    grep -q 'CONFIG_AUTOBOOT_MENUKEY=27' ${rockpro64Uboot}/u-boot.config.fragment
    grep -q 'CONFIG_CMD_SLEEP=y' ${rockpro64Uboot}/u-boot.config.fragment
    grep -q 'CONFIG_FAT_WRITE=y' ${rockpro64Uboot}/u-boot.config.fragment
    grep -q 'preboot=echo "Press ESC during autoboot to enter the bootswain boot menu"' ${rockpro64Uboot}/uboot.env
    grep -q 'bootswain_status=release-candidate' ${rockpro64Uboot}/uboot.env
    grep -q 'bootswain_release_channel=release-candidate' ${rockpro64Uboot}/uboot.env
    grep -a -q 'Press ESC during autoboot to enter the bootswain boot menu' ${rockpro64Uboot}/installer-u-boot.itb
    grep -a -q 'bootmenu_6=Flash SPI firmware=' ${rockpro64Uboot}/installer-u-boot.itb
    grep -a -q 'SPI verify complete' ${rockpro64Uboot}/installer-u-boot.itb
    grep -a -q 'SPI flashing is available only from the installer image' ${rockpro64Uboot}/firmware-u-boot.itb
    grep -q '"default_order": \["emmc", "sd", "usb", "nvme"\]' ${rockpro64ReleaseBundle}/release.json
    grep -q '"one_shot_usb_order": \["usb", "emmc", "sd", "nvme"\]' ${rockpro64ReleaseBundle}/release.json
    grep -q '"one_shot_usb_marker": "/boot/bootswain/next-boot-usb"' ${rockpro64ReleaseBundle}/release.json
    grep -q 'spi.installer.img' ${rockpro64ReleaseBundle}/sha256sums.txt
    grep -q 'shared.disk-image.img' ${rockpro64ReleaseBundle}/sha256sums.txt
    grep -q 'release-notes.md' ${rockpro64ReleaseBundle}/sha256sums.txt
    grep -q 'Hardware validation performed: no' ${rockpro64ReleaseBundle}/release-notes.md
    mkdir -p "$out"
    printf '%s\n' "rockpro64 release-candidate packaging is wired up" > "$out/result"
  '';
in {
  inherit
    validationDir
    qemuArm64Uboot
    raspberryPi3BPlusReleaseCandidate
    raspberryPi3BPlusReleaseBundle
    rockpro64ExperimentalRelease
    rockpro64ReleaseBundle
    rockpro64ReleaseBundleExperimental
    ;

  packages = {
    "qemu-arm64-uboot" = qemuArm64Uboot;
    "rockpro64-uboot" = rockpro64Uboot;
    "rockpro64-spi-firmware" = rockpro64SpiFirmware;
    "rockpro64-spi-installer-img" = rockpro64SpiInstallerImg;
    "rockpro64-spi-installer-img-experimental" = rockpro64SpiInstallerImgExperimental;
    "rockpro64-shared-disk-image-img" = rockpro64SharedDiskImageImg;
    "rockpro64-release-bundle" = rockpro64ReleaseBundle;
    "rockpro64-stable-release-bundle" = rockpro64StableReleaseBundle;
    "rockpro64-release-bundle-experimental" = rockpro64ReleaseBundleExperimental;
    "rockpro64-release-manifest" = rockpro64ReleaseManifest;
    "rockpro64-checksums-provenance" = rockpro64ChecksumsProvenance;
    "raspberrypi3bplus-firmware" = raspberryPi3BPlusFirmwarePackage;
    "raspberrypi3bplus-boot-partition-image" = raspberryPi3BPlusBootPartitionImage;
    "raspberrypi3bplus-release-bundle" = raspberryPi3BPlusReleaseBundle;
    "raspberrypi3bplus-release-manifest" = raspberryPi3BPlusReleaseManifest;
  };

  checks = {
    rockpro64-packaging = rockpro64PackagingCheck;
    raspberrypi3bplus-packaging = raspberryPi3BPlusPackagingCheck;
  };
}
