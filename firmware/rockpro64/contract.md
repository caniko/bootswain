# ROCKPro64 Firmware Contract

This document defines the upstream-first firmware contract for
`pine64-rockpro64` in `bootswain`. It is a planning artifact only.

## Scope

- Maintain a board-specific firmware contract that can later be aligned with
  upstream U-Boot and TF-A sources.
- Publish explicit source fragments for U-Boot configuration, environment, and
  boot script behavior.
- Publish release metadata, checksums, and provenance for the board artifacts.

## Non-goals

- No claim is made that the firmware artifacts are bootable on hardware.
- No claim is made that any SPI layout has been validated on a real board.
- No claim is made that install, erase, or boot flows have been exercised.

## Artifact Contract

- `rockpro64-uboot`: board-oriented U-Boot source bundle and config fragments.
- `rockpro64-spi-firmware`: SPI firmware contract bundle and layout notes.
- `rockpro64-spi-installer-img`: SD installer image scaffold.
- `rockpro64-shared-disk-image-img`: shared-storage image scaffold.
- `rockpro64-release-manifest`: release metadata.
- `checksums/provenance`: checksums and provenance records.

## Release Rule

If a future release claims hardware support, it must add a real validation log
and remove or replace any scaffold-only wording that remains in this tree.
