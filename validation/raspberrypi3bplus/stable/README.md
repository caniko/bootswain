# Raspberry Pi 3 B+ Stable Evidence

Hardware run from 2026-10-29, captured with `bootswain validate serial` over the
GPIO mini-UART (FT232R, 115200 8N1) on a Pi 3 Model B+ (rev 0xa020d3) from a
cold power-on. The required stable scenarios `serial-u-boot-prompt`,
`family-dtb-check` and `sd-extlinux-boot` passed in that single run.

Candidate under test:

- firmware: `/nix/store/68ggbn9l5nf77nx6s221hshbc4jv9432-raspberrypi3bplus-firmware`
  (bootswain 3b7af37; unchanged at the promoting revision)
- SD image: ignitix `example-raspberrypi3bplus` (ignitix 959fc1a with bootswain
  3b7af37), `example-raspberrypi3bplus.img.zst`
  sha256 `ad46a763e8f6df4e9137276077b1766ed7075a73cc8f2c8af455938479f82415`
- U-Boot 2026.07 (`rpi_arm64_defconfig`, prompt `U-Boot> `), Linux 6.18.45 with
  the mainline `bcm2837-rpi-3-b-plus.dtb`, kernel console on `ttyS1`

The leading `: ` bytes in `serial-u-boot-prompt/serial.log` were received while
the board was unpowered, before the U-Boot banner.
