# ROCKPro64 Tow-Boot Phase 0 Release Guide

This document turns the Phase 0 contract into a release procedure for the
ROCKPro64. It is a release-facing document, not a design discussion.

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

If the images are also compressed for distribution, publish the `.zst` versions
and keep the uncompressed names in the manifest.

## Build Procedure

1. Start from a clean checkout in the repository's pinned build environment.
2. Build the board release bundle and collect the artifacts listed above.
3. Generate `sha256sums.txt` and `release.json` from the build outputs.
4. Verify that the manifest records the source revisions and board inputs used
   to produce the release.

## Build Output Rules

The release output must make the board role clear:

- `spi.installer.img` is the SD-bootable installer
- `shared.disk-image.img` is the image for SD or eMMC shared-storage installs
- `idbloader.img`, `u-boot.itb`, and `bl31.elf` are the board-stage firmware
  components used to construct the release

The release manifest should record source revisions, configuration inputs, and
the checksum for each published artifact.

## Install Procedure

### SPI installation path

1. Write `spi.installer.img` to an SD card.
2. Boot the ROCKPro64 from that SD card.
3. In the firmware menu, choose the action that flashes firmware to SPI.
4. Remove the SD card after success.
5. Reboot and verify the board starts from SPI without the installer media.

If the release uses a host-side write step for installer media, `bootswain
flash sd` remains the documented host tool for that job.

### Shared-storage path

1. Write `shared.disk-image.img` to SD or eMMC.
2. Boot the board from that medium.
3. Verify the firmware discovers and boots installed systems from the protected
   firmware partition or reserved span on that device.

The shared-storage path is not a direct SPI replacement. It is a discrete
firmware image placed on removable or internal mass storage.

## Validation Procedure

The release validation set should cover the board states that matter for a
Tow-Boot replacement:

- cold boot with no usable firmware on SPI
- cold boot with SPI firmware already installed
- install firmware to SPI from the installer menu
- reinstall over an existing SPI install
- erase SPI and confirm the board falls through to the next boot source
- boot a UEFI installer from supported media
- boot an extlinux-based image from supported media
- capture serial logs for every trial

If the release claims USB or NVMe support, those paths must appear in the
release validation table. If they are not in the table, they are not claimed.

## Release Notes

The public release notes should answer these questions directly:

- what artifacts were published
- what board revision or board family was validated
- what storage classes were tested
- what boot methods were tested
- what is explicitly unsupported

The release notes should avoid generic claims like "boots most things" and
instead list the actual tested combinations.

## Operator Checks

Before publishing a release, confirm that:

- the checksums match the published artifacts
- the board-specific artifact names are unambiguous
- the SPI installer succeeds from SD media
- the SPI reinstall path works on a board that already has firmware
- the board remains recoverable over serial after a failed or missing boot

## References

- [ROCKPro64 Tow-Boot Phase 0 contract](rockpro64-towboot-phase0-contract.md)
- [Tow-Boot ROCKPro64 device page](https://tow-boot.org/devices/pine64-rockpro64.html)
- [Tow-Boot firmware storage map](https://tow-boot.org/in-depth/firmware-storage-map.html)
- [U-Boot standard boot overview](https://docs.u-boot.org/en/v2025.10/develop/bootstd/overview.html)
