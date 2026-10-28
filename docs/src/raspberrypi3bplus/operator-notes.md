# Raspberry Pi 3 B+ Operator Notes

The release-candidate image is an MBR SD card image with one FAT boot partition,
not a complete NixOS disk image. It provides the board firmware and U-Boot
handoff layer for an extlinux-capable OS partition (the NixOS layer comes from
ignitix).

Layout: partition 1 starts at sector 16384 (8 MiB), type `c`, bootable, 64 MiB,
label `RPI3BOOT`; total image size 72 MiB.

The generated `config.txt`:

```ini
[pi3]
core_freq=250

[all]
kernel=u-boot-rpi3.bin
arm_64bit=1
enable_uart=1
avoid_warnings=1
```

`kernel=` sits in `[all]` so the `[pi02]` and CM3 models boot the same payload.

## Serial wiring

Use a 3.3 V USB-UART adapter only (5 V or RS-232 levels damage the SoC):

| Adapter | Pi 40-pin header |
| --- | --- |
| GND | pin 6 (GND) |
| RXD | pin 8 (GPIO14, TXD) |
| TXD | pin 10 (GPIO15, RXD) |

Settings: 115200 8N1, no flow control. Do not connect the adapter's VCC/5V pin;
power the Pi from its own supply. On the 3B+ the PL011 (`ttyAMA0`) is wired to
Bluetooth, so the header console is the mini-UART (`ttyS1` with the mainline
device tree).

Stable promotion needs captured validation output for the scenarios in
`validation/raspberrypi3bplus/lab-plan.json`; see the [lab runbook](./lab-runbook.md).
