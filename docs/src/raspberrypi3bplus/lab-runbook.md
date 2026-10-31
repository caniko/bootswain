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
  firmware only. There is no OS to boot, so flash it as a smoke test for
  `serial-u-boot-prompt` and `family-dtb-check` only.
- `sd-extlinux-boot` needs an OS image. Use the ignitix
  `example-raspberrypi3bplus` image, or any NixOS image that uses the bootswain
  `raspberryPi3BPlus` module so its firmware partition comes from bootswain.

## Promotion evidence

Stable promotion needs one `validation-run.json` in which `serial-u-boot-prompt`,
`family-dtb-check` and `sd-extlinux-boot` all passed. Produce it with a single run on
an SD card flashed with the OS image above:

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

   The build fails with `missing stable hardware validation evidence` until the
   evidence exists, and with `missing passing stable scenario: <name>` if any of
   the three scenarios did not pass.

Do not stitch evidence from several runs: every run rewrites `validation-run.json`
and marks scenarios it did not select as not-run.

## Debugging single scenarios

`just rpi3bplus-validate-scenario <name>` runs one scenario. It overwrites
`validation-run.json` in its output directory, so use it for debugging only, not as
promotion evidence.

## USB boot

`just rpi3bplus-validate-usb` runs `usb-extlinux-boot` against an OS image on a USB
drive. It is separate and not required for stable promotion.

## Console device

If `login:` does not appear on the header, the OS console is on the wrong UART;
the mainline device tree puts the mini-UART on `ttyS1`, the downstream one on
`ttyS0`. Record the result of the hardware run either way.

## Troubleshooting: what the first hardware run taught us

From the 2026-10-29 bring-up on a Pi 3 Model B+ (rev 0xa020d3) with an FT232R adapter
(`/dev/serial/by-id/usb-FTDI_FT232R_USB_UART_A5069RR4-if00-port0`, which resolved to
`/dev/ttyUSB0`), 115200 8N1.

**Flashing.** `bootswain flash sd` needs root because the device node is `root:disk`.
Double-check the target with `lsblk` (removable, vendor, model); the dry run prints the
same removable/vendor/model fields.

**Total silence on `/dev/ttyUSB0`, not even the U-Boot banner.** TX and RX are crossed
the wrong way. Swap the two data wires: adapter RXD goes to pin 8 (GPIO14, TXD), adapter
TXD to pin 10 (GPIO15, RXD), GND to pin 6, VCC unconnected, adapter set to 3.3 V. To
count pins, pin 1 is the corner pin nearest the SD-card end on the inner row; even pins
are the outer (board-edge) row, so pins 6, 8 and 10 are outer-row positions 3, 4 and 5
from the SD-card end. The firmware stage (`bootcode.bin`, `start.elf`) prints nothing;
the first output is U-Boot about 5 s after power-on.

**A few stray bytes (for example `: : :`) before the banner.** Line noise while the
board is unpowered. Harmless.

**The run passes nothing, or sees only `login:`.** The board was already booted, so every
autoboot-interrupt keypress just re-prints the getty prompt. Validation needs a cold
boot: unplug power, start `just rpi3bplus-validate`, then plug in within the 120 s
window of the first step.

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

**No `login:` on the header.** With
the nixpkgs mainline kernel and `broadcom/bcm2837-rpi-3-b-plus.dtb`, the GPIO mini-UART
is `ttyS1` (16550 at `0x3f215040`) and the PL011 is `ttyAMA0` (Bluetooth). Use
`console=ttyS1,115200n8`; `serial-getty@ttyS1` reaches `login:` about 27 s after
power-on. `ttyS0` applies to the downstream Raspberry Pi kernel and DTB.

**Harmless noise in the boot log.** `Card did not respond to voltage select! : -110`
(the Wi-Fi SDIO slot), a missing `uboot.env`, and `vc4-drm ... Failed to get ddc i2c
adapter` when no HDMI display is attached.

**The board boots but the host is unreachable over the network.** Ethernet is the USB
`lan78xx` NIC, renamed `enu1u1u1`. With no cable the log stops at `configuring for
phy/gmii link mode`. Check the cable before debugging networking.

The NixOS layer is provided by ignitix. CI qualification of this repository
happens in `caniko/fleet`.
