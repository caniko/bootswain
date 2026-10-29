# Raspberry Pi 3 B+ Status and Support Claims

Raspberry Pi 3 B+ support is release-candidate only. Bootswain builds an MBR SD
card image (`sd-card.img`) with an 8 MiB gap and a 64 MiB FAT partition
(`RPI3BOOT`) holding Raspberry Pi boot firmware, the `bcm2710-*` Pi 3 family
device trees, `config.txt`, and `u-boot-rpi3.bin` (mainline `rpi_arm64` U-Boot).

The default boot path is 64-bit Raspberry Pi firmware loading U-Boot, followed by
U-Boot extlinux discovery. The OS layer (the NixOS SD image) is provided by
ignitix; bootswain supplies the firmware set and the flashing tool.

Stable support is not claimed until Raspberry Pi 3 B+ hardware validation logs
are imported under `validation/raspberrypi3bplus/stable/`. See the
[lab runbook](./lab-runbook.md).

## Best-effort family boards

The same firmware set ships DTBs for these boards. They are untested and carry no
support claim:

| Board | DTB | Status |
| --- | --- | --- |
| Raspberry Pi 3 B | `bcm2710-rpi-3-b.dtb` | untested |
| Raspberry Pi 2 B v1.2 | `bcm2710-rpi-2-b.dtb` | untested |
| Compute Module 3 | `bcm2710-rpi-cm3.dtb` | untested |
| Zero 2 W | `bcm2710-rpi-zero-2-w.dtb` | untested |

Unsupported for the release-candidate bundle:

- stable support claims without hardware validation evidence
- stable support for the best-effort family boards
- direct Linux kernel firmware boot
- 32-bit Raspberry Pi boot flow
- EEPROM-style update workflows
- network boot
