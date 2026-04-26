# bootswain generic firmware boot menu template.
#
# Boards provide a fragment at the marker below. The fragment is expected to
# define board identity, boot target mapping, and any board-specific
# maintenance actions needed by the generic menu entries.

if test -n "${bootswain_runtime_role}"; then
	setenv bootswain_chainloader_role "${bootswain_runtime_role}"
fi
if test "${bootswain_has_bootmenu}" = "1"; then
	setenv bootswain_chainloader_has_bootmenu 1
fi
if test -n "${devtype}"; then
	setenv bootswain_chainload_devtype "${devtype}"
fi
if test -n "${devnum}"; then
	setenv bootswain_chainload_devnum "${devnum}"
fi
if test -n "${distro_bootpart}"; then
	setenv bootswain_chainload_bootpart "${distro_bootpart}"
fi
if test -z "${devtype}"; then
	setenv devtype mmc
fi
if test -z "${devnum}"; then
	setenv devnum 1
fi
if test -z "${distro_bootpart}"; then
	setenv distro_bootpart 1
fi
if test -z "${prefix}"; then
	setenv prefix /
fi
if test -z "${bootswain_bootmenu_delay}"; then
	setenv bootswain_bootmenu_delay 30
fi
if test -z "${bootswain_boot_scan_policy}"; then
	setenv bootswain_boot_scan_policy manual
fi

@@BOOTSWAIN_BOARD_FRAGMENT@@

if test -z "${bootswain_menu_title}"; then
	setenv bootswain_menu_title 'bootswain firmware menu'
fi
if test -z "${bootswain_ready_message}"; then
	setenv bootswain_ready_message 'bootswain firmware menu ready'
fi
if test -z "${bootswain_flash_spi}"; then
	setenv bootswain_flash_spi 'echo "SPI flashing is not available on this build"; false'
fi
if test -z "${bootswain_erase_spi}"; then
	setenv bootswain_erase_spi 'echo "SPI erase is not available on this build"; false'
fi
if test -z "${bootswain_boot_prepare_emmc}"; then
	setenv bootswain_boot_prepare_emmc 'true'
fi
if test -z "${bootswain_boot_prepare_sd}"; then
	setenv bootswain_boot_prepare_sd 'true'
fi
if test -z "${bootswain_boot_prepare_usb}"; then
	setenv bootswain_boot_prepare_usb 'usb start; true'
fi
if test -z "${bootswain_boot_prepare_nvme}"; then
	setenv bootswain_boot_prepare_nvme 'pci enum; nvme scan; true'
fi

echo "${bootswain_menu_title}"
echo "channel=${bootswain_release_channel}"
echo "board=${bootswain_board}"
echo "soc=${bootswain_soc}"

if test "${bootswain_boot_scan_policy}" = fast; then
	setenv bootswain_detected_menu_label 'Detected boot options'
else
	setenv bootswain_detected_menu_label 'Rescan detected boot options'
fi

setenv bootswain_boot_prepare_all 'usb start; pci enum; nvme scan; true'
setenv bootswain_boot_auto 'if run bootswain_boot_emmc; then true; elif run bootswain_boot_sd; then true; elif run bootswain_boot_usb; then true; elif run bootswain_boot_nvme; then true; else echo "No bootable media"; false; fi'
setenv bootswain_boot_detected_menu 'run bootswain_boot_prepare_all; echo "Scanning for detected boot options"; bootflow scan -m -b; echo "No detected boot options"; false'
setenv bootswain_boot_emmc 'run bootswain_boot_prepare_emmc; echo "Scanning eMMC bootflows"; bootflow scan -lb ${bootswain_boot_target_emmc}; echo "No bootable media on eMMC"; false'
setenv bootswain_boot_sd 'run bootswain_boot_prepare_sd; echo "Scanning SD bootflows"; bootflow scan -lb ${bootswain_boot_target_sd}; echo "No bootable media on SD"; false'
setenv bootswain_boot_usb 'run bootswain_boot_prepare_usb; echo "Scanning USB bootflows"; bootflow scan -lb ${bootswain_boot_target_usb}; echo "No bootable media on USB"; false'
setenv bootswain_boot_nvme 'run bootswain_boot_prepare_nvme; echo "Scanning NVMe bootflows"; bootflow scan -lb ${bootswain_boot_target_nvme}; echo "No bootable media on NVMe"; false'

setenv bootswain_menu 'if test "${bootswain_has_bootmenu}" = "1"; then bootmenu ${bootswain_bootmenu_delay}; else run bootswain_menu_fallback; fi'
setenv bootswain_menu_fallback 'echo "*** U-Boot Boot Menu ***"; echo "1. Continue boot"; echo "2. ${bootswain_detected_menu_label}"; echo "3. Boot from eMMC"; echo "4. Boot from SD"; echo "5. Boot from USB"; echo "6. Boot from NVMe"; echo "7. Flash SPI firmware"; echo "8. Erase SPI firmware"; echo "9. Enter U-Boot shell"; echo "bootmenu command unavailable; serial command mode active"; echo "Run: run bootswain_flash_spi"; echo "Run: run bootswain_erase_spi"; echo "Run: run bootswain_boot_auto"; echo "bootswain serial command mode ready"'
setenv bootswain_auto_upgrade_spi 'echo "Auto-upgrading SPI firmware from legacy chainloader"; if run bootswain_flash_spi; then echo "SPI install complete; rebooting"; reset; else echo "SPI install failed"; false; fi'
setenv bootmenu_title "${bootswain_menu_title}"
setenv bootmenu_0 'Continue boot=if run bootswain_boot_auto; then true; else run bootswain_menu; fi'
setenv bootmenu_1 "${bootswain_detected_menu_label}=if run bootswain_boot_detected_menu; then true; else run bootswain_menu; fi"
setenv bootmenu_2 'Boot from eMMC=if run bootswain_boot_emmc; then true; else run bootswain_menu; fi'
setenv bootmenu_3 'Boot from SD=if run bootswain_boot_sd; then true; else run bootswain_menu; fi'
setenv bootmenu_4 'Boot from USB=if run bootswain_boot_usb; then true; else run bootswain_menu; fi'
setenv bootmenu_5 'Boot from NVMe=if run bootswain_boot_nvme; then true; else run bootswain_menu; fi'
setenv bootmenu_6 'Flash SPI firmware=if run bootswain_flash_spi; then echo "SPI flash action succeeded"; else echo "SPI flash action failed"; fi; sleep 3; run bootswain_menu'
setenv bootmenu_7 'Erase SPI firmware=if run bootswain_erase_spi; then echo "SPI erase action succeeded"; else echo "SPI erase action failed"; fi; sleep 3; run bootswain_menu'
setenv bootmenu_8 'Enter U-Boot shell=echo "Dropping to U-Boot shell"'

echo "${bootswain_ready_message}"
if test -n "${bootswain_spi_image}"; then
	echo "SPI payload: ${bootswain_spi_image}"
fi
if test -n "${bootswain_spi_image_fallback}"; then
	echo "SPI payload fallback: ${bootswain_spi_image_fallback}"
fi
if test -n "${bootswain_spi_source_devtype}"; then
	echo "SPI payload source: ${bootswain_spi_source_devtype} ${bootswain_spi_source_devnum}:${bootswain_spi_source_bootpart}"
fi
echo "*** U-Boot Boot Menu ***"
echo "1. Continue boot"
echo "2. ${bootswain_detected_menu_label}"
echo "3. Boot from eMMC"
echo "4. Boot from SD"
echo "5. Boot from USB"
echo "6. Boot from NVMe"
echo "7. Flash SPI firmware"
echo "8. Erase SPI firmware"
echo "9. Enter U-Boot shell"
if test "${bootswain_script_mode}" = "validation-shell"; then
	echo "bootswain validation shell ready"
	setenv bootswain_script_mode
elif test "${bootswain_chainloader_role}" = "firmware"; then
	echo "BootsWain installer media detected under installed firmware"
	run bootswain_menu
elif test -z "${bootswain_chainloader_role}"; then
	if test "${bootswain_chainloader_has_bootmenu}" = "1"; then
		echo "BootsWain installer media detected under installed firmware"
		run bootswain_menu
	else
		run bootswain_auto_upgrade_spi
	fi
elif test "${bootswain_chainloader_role}" = "installer"; then
	run bootswain_menu
else
	run bootswain_auto_upgrade_spi
fi
