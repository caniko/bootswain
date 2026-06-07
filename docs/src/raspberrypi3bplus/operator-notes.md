# Raspberry Pi 3 B+ Operator Notes

The release-candidate image is a FAT boot partition, not a complete NixOS disk
image. It is intended to provide the board firmware and U-Boot handoff layer for
an extlinux-capable OS partition.

The generated `config.txt` follows the NixOS aarch64 Raspberry Pi convention:

```ini
[pi3]
kernel=u-boot-rpi3.bin
core_freq=250

[all]
arm_64bit=1
enable_uart=1
avoid_warnings=1
```

Use serial at 115200 baud on the Pi UART for validation. Stable promotion needs
captured validation output for the deferred scenarios in
`validation/raspberrypi3bplus/lab-plan.json`.
