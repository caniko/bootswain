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
- serial kernel parameters for `ttyAMA0` and the local console

The firmware package installs these files to the configured boot mount:

- `bootcode.bin`
- `start*.elf`
- `fixup*.dat`
- `bcm2710-rpi-3-b-plus.dtb`
- `u-boot-rpi3.bin`
- `config.txt`

The boot mount defaults to `/boot` and can be changed with
`boot.bootswain.raspberryPi3BPlus.bootMountPoint`.
