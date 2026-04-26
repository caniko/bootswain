# ROCKPro64 Release Guide

This procedure turns the Phase 0 contract into a release process for the
ROCKPro64.

## Release Inputs

The release is expected to be produced from a clean checkout with pinned board
inputs. The published release bundle should include:

- `spi.installer.img`
- `shared.disk-image.img`
- `idbloader.img`
- `u-boot.itb`
- `bl31.elf`
- `sha256sums.txt`
- `release.json`
- `release-notes.md`

If images are compressed for distribution, publish the `.zst` versions and keep
the uncompressed names in the manifest.

## Build Procedure

1. Start from a clean checkout in the repository's pinned build environment.
2. Build the board release-candidate bundle and collect the artifacts listed above.
3. Generate `sha256sums.txt` and `release.json` from the build outputs.
4. Verify that the manifest records the source revisions and board inputs used
   to produce the release.
5. Promote to stable only after importing hardware evidence and building
   `.#rockpro64-stable-release-bundle`.

## Install Procedure

### SPI Installation

1. Write `spi.installer.img` to an SD card.
2. Boot the ROCKPro64 from that SD card.
3. In the firmware menu, choose the action that flashes firmware to SPI.
4. Remove the SD card after success.
5. Reboot and verify the board starts from SPI without installer media.

If the release uses a host-side write step for installer media,
`bootswain flash sd` remains the documented host tool for that job.

### Shared Storage

1. Write `shared.disk-image.img` to SD or eMMC.
2. Boot the board from that medium.
3. Verify the firmware discovers and boots installed systems from the protected
   firmware partition or reserved span on that device.

The shared-storage path is not a direct SPI replacement. It is a discrete
firmware image placed on removable or internal mass storage.

## Validation Procedure

The release validation set covers:

- cold boot with no usable firmware on SPI
- cold boot with SPI firmware already installed
- install firmware to SPI from the installer menu
- reinstall over an existing SPI install
- erase SPI and confirm the board falls through to the next boot source
- boot a UEFI installer from supported media
- boot an extlinux-based image from supported media
- capture serial logs for every trial

If the release claims USB, SD, eMMC, or NVMe support, those paths must appear in
the release validation table. If they are not in the table, they are not
claimed.

## Operator Checks

Before publishing a release, confirm that:

- checksums match the published artifacts
- board-specific artifact names are unambiguous
- firmware maps eMMC to `mmc0` and SD to `mmc1` on the tested ROCKPro64
- SPI installer succeeds from SD media
- SPI reinstall works on a board that already has firmware
- the board remains recoverable over serial after a failed or missing boot
- the stable bundle contains passing evidence for every scenario named by its
  validation claims
