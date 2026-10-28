# Raspberry Pi 3 B+ Release Guide

Build the release-candidate bundle:

```sh
nix build .#raspberrypi3bplus-release-bundle
```

Flash the SD card image through the convenience wrapper:

```sh
nix run .#flash -- \
  --target raspberrypi3bplus-sd-card \
  --device /dev/sdX \
  --verify
```

Aliases: `raspberrypi3bplus`, `rpi3bplus`, `raspberrypi3bplus-boot-partition`,
`rpi-3-b-plus`, `raspberry-pi-3-b-plus`.

The bundle contains:

- `sd-card.img` (artifact kind `sd-card-image`, flashable)
- `firmware/`
- `raspberry-pi-firmware.tar`
- `u-boot-rpi3.bin`
- `release.json` (includes `best_effort_boards`, untested)
- `sha256sums.txt`
- `provenance.json`
- `release-notes.md`

The release remains a release candidate until hardware validation evidence is
imported under `validation/raspberrypi3bplus/stable/`. Then:

```sh
nix build .#raspberrypi3bplus-stable-release-bundle
```

This requires `validation-run.json` with the scenarios `serial-u-boot-prompt`,
`family-dtb-check` and `sd-extlinux-boot` passed, plus their serial logs. See the
[lab runbook](../raspberrypi3bplus/lab-runbook.md).
