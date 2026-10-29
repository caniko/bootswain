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

The NixOS layer is provided by ignitix. CI qualification of this repository
happens in `caniko/fleet`.
