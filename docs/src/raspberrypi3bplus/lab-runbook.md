# Raspberry Pi 3 B+ Lab Runbook

Hardware: a Raspberry Pi 3 B+, a 3.3 V USB-UART adapter, and an SD card.

## Wiring

Use a 3.3 V adapter only; do not connect its VCC/5V pin. Power the Pi from its own supply.

| Adapter | Pi 40-pin header |
| --- | --- |
| GND | pin 6 (GND) |
| RXD | pin 8 (GPIO14, TXD) |
| TXD | pin 10 (GPIO15, RXD) |

Serial settings: 115200 8N1, no flow control.

## Which image proves what

- `raspberrypi3bplus-sd-card` (`sd-card.img`, 72 MiB, one FAT partition) is
  firmware only. With no OS to boot, it can only smoke-test
  `serial-u-boot-prompt` and `family-dtb-check`.
- `sd-extlinux-boot` needs an OS image. Use the ignitix
  `example-raspberrypi3bplus` image or a NixOS image that uses Bootswain's
  `raspberryPi3BPlus` module for its firmware partition.

## Promotion evidence

For stable promotion, pass `serial-u-boot-prompt`, `family-dtb-check` and
`sd-extlinux-boot` in one run on an SD card with the OS image above. Record all
three in one `validation-run.json`:

1. Flash the OS image to the SD card and put it in the Pi.
2. Connect the adapter, power the board, and run:

   ```sh
   just rpi3bplus-validate
   ```

   Equivalent without `just`:

   ```sh
   bootswain validate serial \
     --plan validation/raspberrypi3bplus/lab-plan.json \
     --scenario serial-u-boot-prompt \
     --scenario family-dtb-check \
     --scenario sd-extlinux-boot
   ```

3. Copy `validation-run.json` and the serial logs from the output directory into
   `validation/raspberrypi3bplus/stable/`.
4. Promote:

   ```sh
   nix build .#raspberrypi3bplus-stable-release-bundle
   ```

    The build fails with `missing stable hardware validation evidence` if
    evidence is absent, or `missing passing stable scenario: <name>` if any of
    the three scenarios did not pass.

Do not stitch evidence from several runs: every run rewrites `validation-run.json`
and marks scenarios it did not select as not-run.

## Debugging single scenarios

Use `just rpi3bplus-validate-scenario <name>` only to debug one scenario. It
overwrites `validation-run.json` in its output directory; do not use it as
promotion evidence.

## USB boot

`just rpi3bplus-validate-usb` runs `usb-extlinux-boot` against an OS image on a USB
drive. This separate run is not required for stable promotion.

## Console device

If the header never shows `login:`, the OS console is on the wrong UART. The
mainline device tree puts the mini-UART on `ttyS1`; the downstream tree uses
`ttyS0`. Record the hardware result either way.

## Troubleshooting: what the first hardware run taught us

2026-10-29 bring-up: Pi 3 Model B+ (rev 0xa020d3), FT232R adapter at 115200 8N1.
`/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_A5069RR4-if00-port0` resolved to
`/dev/ttyUSB0`.

**Flashing.** `bootswain flash sd` needs root because the device node is `root:disk`.
Check the target's removable/vendor/model fields with `lsblk` and the dry run.

**No output on `/dev/ttyUSB0`, not even the U-Boot banner.** TX and RX are crossed
the wrong way. Swap the data wires: adapter RXD goes to pin 8 (GPIO14, TXD), adapter
TXD to pin 10 (GPIO15, RXD), GND to pin 6, VCC unconnected, adapter set to 3.3 V. To
count pins, pin 1 is the corner pin nearest the SD-card end on the inner row; even pins
are the outer (board-edge) row, so pins 6, 8 and 10 are outer-row positions 3, 4 and 5
from the SD-card end. The firmware stage (`bootcode.bin`, `start.elf`) prints nothing;
the first output is U-Boot about 5 s after power-on.

**A few stray bytes (for example `: : :`) before the banner.** Harmless line
noise while the board is unpowered.

**No scenario passes, or only `login:` appears.** The board already booted, so
each autoboot-interrupt keypress repeats the getty prompt. Cold boot for
validation: unplug power, start `just rpi3bplus-validate`, then reconnect power
within the first step's 120 s window.

**The U-Boot prompt is never matched.** nixpkgs' `rpi_arm64` U-Boot sets
`CONFIG_SYS_PROMPT="U-Boot> "`, not `=> `. The plan sets the top-level
`"uboot_prompt": "U-Boot> "`; the validator default stays `=> `.

**`fdt print /model` fails with `FDT_ERR_NOTFOUND`.** `model` is a property of `/`, not a
node. Use `fdt addr ${fdt_addr}; fdt print / model`. The firmware rewrites `/model` at
boot to `Raspberry Pi 3 Model B Plus Rev 1.3` (the DTS says `Model B+`), so the check
matches `Raspberry Pi 3 Model B Plus` plus the stable compatible
`raspberrypi,3-model-b-plus`.

**`bootflow scan -lb mmc0` prints `Flags not supported: enable CONFIG_BOOTSTD_FULL`.**
This U-Boot cannot select a device that way. Use `setenv boot_targets mmc; bootflow
scan`, or for USB `setenv boot_targets usb; usb start; bootflow scan`.

**`Boot failed (err=-14)` right after `** Booting bootflow '<NULL>' with efi_mgr`.** Not
fatal. bootstd tries the EFI boot manager first, then boots the extlinux bootflow on
partition 2. The validator does not treat that line as terminal
(`TERMINAL_FAILURE_MARKERS` in `crates/bootswain-cli/src/uboot_serial/read.rs`).

**No `login:` on the header.** With the nixpkgs mainline kernel and
`broadcom/bcm2837-rpi-3-b-plus.dtb`, the GPIO mini-UART
is `ttyS1` (16550 at `0x3f215040`) and the PL011 is `ttyAMA0` (Bluetooth). Use
`console=ttyS1,115200n8`; `serial-getty@ttyS1` reaches `login:` about 27 s after
power-on. `ttyS0` applies to the downstream Raspberry Pi kernel and DTB.

**Harmless noise in the boot log.** `Card did not respond to voltage select! : -110`
(the Wi-Fi SDIO slot), a missing `uboot.env`, and `vc4-drm ... Failed to get ddc i2c
adapter` when no HDMI display is attached.

**The board boots but the host is unreachable over the network.** Ethernet is the USB
`lan78xx` NIC, renamed `enu1u1u1`. With no cable the log stops at `configuring for
phy/gmii link mode`. Check the cable before debugging networking.

Ignitix provides the NixOS layer. CI qualification runs in `caniko/fleet`.
