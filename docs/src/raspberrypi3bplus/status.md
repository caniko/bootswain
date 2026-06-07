# Raspberry Pi 3 B+ Status and Support Claims

Raspberry Pi 3 B+ support is release-candidate only. Bootswain builds a FAT boot
partition image with Raspberry Pi boot firmware, `bcm2710-rpi-3-b-plus.dtb`,
`config.txt`, and `u-boot-rpi3.bin`.

The default boot path is 64-bit Raspberry Pi firmware loading U-Boot, followed by
U-Boot extlinux discovery.

Stable support is not claimed until Raspberry Pi 3 B+ hardware validation logs
are imported under `validation/raspberrypi3bplus/stable/`.

Unsupported for the release-candidate bundle:

- stable support claims without hardware validation evidence
- direct Linux kernel firmware boot
- 32-bit Raspberry Pi boot flow
- EEPROM-style update workflows
- network boot
