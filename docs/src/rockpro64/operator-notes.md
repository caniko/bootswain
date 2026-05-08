# ROCKPro64 Operator Notes

These notes capture hardware findings from hands-on ROCKPro64 bring-up with
bootswain SPI firmware and a downstream NixOS install.

## MMC Numbering

On the tested ROCKPro64 v2.1 with U-Boot 2026.04, U-Boot enumerated the useful
MMC devices as:

- `mmc0`: eMMC controller at `mmc@fe330000`
- `mmc1`: SD controller at `mmc@fe320000`

A serial check made the mapping visible: `mmc dev 1; mmc part` showed a single
partition named `disk-sdcard-storage`, which matched the downstream SD-card
backing partition, not the eMMC layout. The firmware mapping must therefore
use:

```text
bootswain_boot_target_emmc=mmc0
bootswain_boot_target_sd=mmc1
boot_targets=mmc0 mmc1 usb0 nvme0 pxe dhcp
```

When diagnosing a board, confirm with:

```text
mmc dev 0
mmc part
mmc dev 1
mmc part
```

## Bootflow Behavior

`bootflow scan -ale mmc0` on the corrected eMMC target found one valid bootflow:

```text
efi ready mmc 1 ... /EFI/BOOT/BOOTAA64.EFI
```

To boot it manually from the U-Boot prompt:

```text
bootflow select 6
bootflow boot
```

or let U-Boot scan and boot the first valid eMMC flow:

```text
bootflow scan -lb mmc0
```

The diagnostic command `bootflow scan -ale <label>` lists failed methods and
partitions. The boot command `bootflow scan -lb <label>` is what the firmware
menu should use for actual boot attempts.

A `bootflow scan -b` command may return to the script after listing zero valid
flows, even when `/EFI/BOOT/BOOTAA64.EFI` is present and direct `bootefi` works.
If control returns to bootswain after a booting scan, treat that path as not
booted and continue fallback or return to the menu. The installed NixOS handoff
therefore attempts bootflow first, then directly loads the removable EFI binary
from the known ESP partition and runs `bootefi`.

## One-Shot USB Boot

The installed NixOS handoff normally scans `eMMC -> SD -> USB -> NVMe`. To boot
USB first for one reboot, create this marker on the eMMC ESP before rebooting:

```text
/boot/bootswain/next-boot-usb
```

The NixOS module provides the helper:

```text
sudo bootswain-reboot-usb
```

U-Boot checks `mmc 0:1` for `/bootswain/next-boot-usb`, removes the marker with
`fatrm`, then scans `USB -> eMMC -> SD -> NVMe`. If the marker cannot be
removed, bootswain falls back to the normal boot order to avoid a persistent USB
boot loop.

## EFI vs Extlinux

U-Boot can support both EFI and extlinux boot methods at the firmware level.
NixOS normally expects exactly one bootloader installer for a system closure.

- Use `boot.loader.systemd-boot.enable = true` when the target has a mounted
  VFAT ESP and should expose `/EFI/BOOT/BOOTAA64.EFI`.
- Use `boot.loader.generic-extlinux-compatible.enable = true` when the target
  should expose `/boot/extlinux/extlinux.conf`.

For an eMMC GPT with a VFAT ESP mounted at `/boot`, EFI/systemd-boot is the
first-class path:

```nix
boot.bootswain.rockpro64 = {
  enable = true;
  osBootProtocol = "efi";
};
```

Use `osBootProtocol = "extlinux"` only for images that intentionally expose an
extlinux bootflow.

## Useful Serial Checks

At a U-Boot prompt, these commands quickly distinguish mapping, filesystem, and
boot method problems:

```text
mmc list
mmc dev 0
mmc part
fatls mmc 0:1 /
fatls mmc 0:1 /EFI/BOOT
fatload mmc 0:1 ${kernel_addr_r} /EFI/BOOT/BOOTAA64.EFI
bootefi ${kernel_addr_r} ${fdtcontroladdr}
bootflow scan -ale mmc0
bootflow scan -lb mmc0
```

If the partition table shown for the supposed eMMC contains only
`disk-sdcard-storage`, the firmware is scanning the SD device.

## SPI Firmware Update Reminder

Local NixOS changes do not change already-installed SPI firmware. After fixing
bootswain's firmware scripts or MMC mapping, rebuild the SPI firmware bundle
and reflash SPI before expecting the default boot menu to use the new mapping.
