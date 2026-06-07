# Raspberry Pi 3 B+ Release Guide

Build the release-candidate bundle:

```sh
nix build .#raspberrypi3bplus-release-bundle
```

Flash the boot partition image through the convenience wrapper:

```sh
nix run .#flash -- \
  --target raspberrypi3bplus-boot-partition \
  --device /dev/sdX \
  --verify
```

Aliases are available for `raspberrypi3bplus` and `rpi3bplus`.

The bundle contains:

- `boot-partition.img`
- `firmware/`
- `raspberry-pi-firmware.tar`
- `u-boot-rpi3.bin`
- `release.json`
- `sha256sums.txt`
- `provenance.json`
- `release-notes.md`

The release remains a release candidate until hardware validation evidence is
imported under `validation/raspberrypi3bplus/stable/`.
