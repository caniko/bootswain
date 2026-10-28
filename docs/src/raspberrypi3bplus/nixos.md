# Raspberry Pi 3 B+ NixOS Integration

Use the bootable module for an aarch64 Raspberry Pi 3 B+ system:

```nix
{
  imports = [
    bootswain.nixosModules.raspberryPi3BPlusBootable
  ];
}
```

The module defaults to:

- `nixpkgs.hostPlatform = "aarch64-linux"`
- extlinux enabled
- GRUB and systemd-boot disabled
- serial kernel parameters `console=ttyS1,115200n8` (the header mini-UART; the
  3B+ PL011 is Bluetooth) followed by `console=tty0`

The firmware package installs these files to the configured boot mount:

- `bootcode.bin`
- `start*.elf`
- `fixup*.dat`
- every `bcm2710-*.dtb` (Pi 2 B v1.2, 3 B, 3 B+, CM3, Zero 2, Zero 2 W)
- `u-boot-rpi3.bin`
- `config.txt`

The boot mount defaults to `/boot` and can be changed with
`boot.bootswain.raspberryPi3BPlus.bootMountPoint`. If it is not a mountpoint at
activation time, installation is skipped with a warning instead of writing into
the bare directory.

`boot.bootswain.raspberryPi3BPlus.firmwarePackage` defaults to the flake's
`raspberrypi3bplus-firmware` for the build platform and can be overridden.
