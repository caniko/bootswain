# SPDX-License-Identifier: MIT OR Apache-2.0
# bootswain ROCKPro64 board fragment for the generic firmware boot menu.

setenv bootswain_board pine64-rockpro64
setenv bootswain_soc rockchip-rk3399
setenv bootswain_status release-candidate
setenv bootswain_validation hardware-required
setenv bootswain_release_stage release-candidate
setenv bootswain_release_channel release-candidate
setenv bootswain_runtime_role installer

setenv bootswain_menu_title 'bootswain ROCKPro64 SPI installer'
setenv bootswain_ready_message 'ROCKPro64 release-candidate installer ready'
setenv bootswain_bootmenu_delay 5
setenv bootswain_boot_scan_policy manual

setenv bootswain_boot_target_emmc mmc0
setenv bootswain_boot_target_sd mmc1
setenv bootswain_boot_target_usb usb0
setenv bootswain_boot_target_nvme nvme0

setenv bootswain_boot_prepare_emmc 'true'
setenv bootswain_boot_prepare_sd 'true'
setenv bootswain_boot_prepare_usb 'usb start; true'
setenv bootswain_boot_prepare_nvme 'pci enum; nvme scan; true'

if test -z "${fdtfile}"; then
	setenv fdtfile rockchip/rk3399-rockpro64.dtb
fi
if test -z "${scriptaddr}"; then
	setenv scriptaddr 0x00c00000
fi
if test -z "${pxefile_addr_r}"; then
	setenv pxefile_addr_r 0x00e00000
fi
if test -z "${kernel_addr_r}"; then
	setenv kernel_addr_r 0x02000000
fi
if test -z "${kernel_comp_addr_r}"; then
	setenv kernel_comp_addr_r 0x0a000000
fi
if test -z "${fdt_addr_r}"; then
	setenv fdt_addr_r 0x12000000
fi
if test -z "${fdtoverlay_addr_r}"; then
	setenv fdtoverlay_addr_r 0x12100000
fi
if test -z "${ramdisk_addr_r}"; then
	setenv ramdisk_addr_r 0x12180000
fi
if test -z "${kernel_comp_size}"; then
	setenv kernel_comp_size 0x8000000
fi

if test -n "${bootswain_chainload_devtype}"; then
	setenv bootswain_spi_source_devtype "${bootswain_chainload_devtype}"
else
	setenv bootswain_spi_source_devtype mmc
fi
if test -n "${bootswain_chainload_devnum}"; then
	setenv bootswain_spi_source_devnum "${bootswain_chainload_devnum}"
else
	setenv bootswain_spi_source_devnum 0
fi
if test -n "${bootswain_chainload_bootpart}"; then
	setenv bootswain_spi_source_bootpart "${bootswain_chainload_bootpart}"
else
	setenv bootswain_spi_source_bootpart 1
fi
setenv bootswain_spi_image /u-boot-rockchip-spi.bin
setenv bootswain_spi_image_fallback /boot/u-boot-rockchip-spi.bin
setenv bootswain_spi_size 0x1000000
setenv bootswain_spi_max_payload_size 0x1000000
setenv bootswain_spi_verify_addr 0x18000000
setenv bootswain_expected_compatible pine64,rockpro64
setenv bootswain_expected_compatible_v2_1 pine64,rockpro64-v2.1

setenv bootswain_check_board 'echo "Checking ROCKPro64 board identity"; if fdt addr ${fdtcontroladdr}; then if fdt get value bootswain_detected_compatible / compatible; then if test "${bootswain_detected_compatible}" = "${bootswain_expected_compatible}"; then true; else if test "${bootswain_detected_compatible}" = "${bootswain_expected_compatible_v2_1}"; then true; else echo "Unexpected board compatible: ${bootswain_detected_compatible}"; false; fi; fi; else echo "Unable to read control FDT compatible"; false; fi; else echo "Unable to access control FDT"; false; fi'
setenv bootswain_probe_spi 'echo "Probing SPI flash"; if sf probe; then if test -n "${sf_size}"; then if test "${sf_size}" = "${bootswain_spi_size}"; then true; else echo "Unexpected SPI size: ${sf_size}"; false; fi; else echo "SPI size variable unavailable; continuing with configured 16 MiB bound"; true; fi; else echo "SPI probe failed"; false; fi'
setenv bootswain_load_spi_payload 'echo "Loading SPI payload from ${bootswain_spi_source_devtype} ${bootswain_spi_source_devnum}:${bootswain_spi_source_bootpart}"; if fatload ${bootswain_spi_source_devtype} ${bootswain_spi_source_devnum}:${bootswain_spi_source_bootpart} ${kernel_addr_r} ${bootswain_spi_image}; then true; else echo "Primary SPI payload not found at ${bootswain_spi_image}; trying ${bootswain_spi_image_fallback}"; if fatload ${bootswain_spi_source_devtype} ${bootswain_spi_source_devnum}:${bootswain_spi_source_bootpart} ${kernel_addr_r} ${bootswain_spi_image_fallback}; then true; else echo "Failed to load SPI payload from ${bootswain_spi_source_devtype} ${bootswain_spi_source_devnum}:${bootswain_spi_source_bootpart}"; false; fi; fi'
setenv bootswain_check_spi_payload 'echo "Checking SPI payload size ${filesize}"; if itest ${filesize} -gt 0; then if itest ${filesize} -le ${bootswain_spi_max_payload_size}; then true; else echo "SPI payload is too large for configured flash bound"; false; fi; else echo "SPI payload size is empty"; false; fi'
setenv bootswain_write_spi_payload 'echo "Writing SPI payload"; if sf update ${kernel_addr_r} 0 ${filesize}; then true; else echo "SPI flash write failed"; false; fi'
setenv bootswain_verify_spi_payload 'echo "Verifying SPI payload"; if sf read ${bootswain_spi_verify_addr} 0 ${filesize}; then if cmp.b ${kernel_addr_r} ${bootswain_spi_verify_addr} ${filesize}; then echo "SPI verify complete"; true; else echo "SPI verify mismatch"; false; fi; else echo "SPI verify read failed"; false; fi'
setenv bootswain_refuse_installed_spi_action 'if test "${bootswain_chainloader_role}" = "firmware"; then true; else if test -z "${bootswain_chainloader_role}"; then if test "${bootswain_chainloader_has_bootmenu}" = "1"; then true; else false; fi; else false; fi; fi'
setenv bootswain_flash_spi 'if run bootswain_refuse_installed_spi_action; then echo "SPI flashing is refused from installed SPI firmware"; false; else echo "Flashing SPI payload from ${bootswain_spi_source_devtype} ${bootswain_spi_source_devnum}:${bootswain_spi_source_bootpart}"; if run bootswain_check_board; then if ${bootswain_spi_source_devtype} dev ${bootswain_spi_source_devnum}; then if ${bootswain_spi_source_devtype} rescan; then if run bootswain_probe_spi; then if run bootswain_load_spi_payload; then if run bootswain_check_spi_payload; then if run bootswain_write_spi_payload; then if run bootswain_verify_spi_payload; then echo "SPI flash complete"; else false; fi; else false; fi; else false; fi; else false; fi; else false; fi; else echo "Installer source rescan failed"; false; fi; else echo "Failed to select installer source"; false; fi; else false; fi; fi'
setenv bootswain_erase_spi 'if run bootswain_refuse_installed_spi_action; then echo "SPI flashing is refused from installed SPI firmware"; false; else echo "Erasing SPI flash"; if run bootswain_check_board; then if run bootswain_probe_spi; then if sf erase 0 ${bootswain_spi_size}; then echo "SPI erase complete"; true; else echo "SPI erase command failed"; false; fi; else false; fi; else false; fi; fi'
