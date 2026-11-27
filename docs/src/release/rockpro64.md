# ROCKPro64 Release Guide

Use this procedure to release ROCKPro64 firmware under the Phase 0 contract.

## Release Inputs

Build from a clean checkout with pinned board inputs. The published bundle
should include:

- `spi.installer.img`
- `shared.disk-image.img`
- `idbloader.img`
- `u-boot.itb`
- `bl31.elf`
- `sha256sums.txt`
- `release.json`
- `release-notes.md`

If you compress images for distribution, publish the `.zst` versions and keep
the uncompressed names in the manifest.

## Build Procedure

1. Start from a clean checkout in the repository's pinned build environment.
2. Build the board release-candidate bundle and collect the artifacts listed above.
3. Generate `sha256sums.txt` and `release.json` from the build outputs.
4. Verify the manifest records the release's source revisions and board inputs.
5. Promote to stable only after importing hardware evidence and building
   `.#rockpro64-stable-release-bundle`.

## Install Procedure

### SPI Installation

1. Write `spi.installer.img` to an SD card.
2. Boot the ROCKPro64 from that SD card.
3. Choose the firmware menu action that flashes firmware to SPI.
4. Remove the SD card after success.
5. Reboot and verify the board starts from SPI without installer media.

If the release calls for writing installer media from a host, use
`bootswain flash sd`.

### Shared Storage

1. Write `shared.disk-image.img` to SD or eMMC.
2. Boot the board from that medium.
3. Verify the firmware discovers and boots installed systems from the protected
   firmware partition or reserved span on that device.

Shared storage puts a discrete firmware image on removable or internal mass
storage; it does not directly replace SPI.

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

List each claimed USB, SD, eMMC, or NVMe path in the release validation table.
Paths absent from the table are unclaimed.

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
