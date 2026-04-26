# ROCKPro64 Firmware Contract

This document defines the upstream-first firmware contract for
`pine64-rockpro64` in `bootswain`. It is a planning artifact only.

## Scope

- Maintain a board-specific firmware contract that can later be aligned with
  upstream U-Boot and TF-A sources.
- Publish explicit source fragments for U-Boot configuration, environment, and
  boot script behavior.
- Publish release metadata, checksums, and provenance for the board artifacts.
- Produce release-candidate SD-bootable SPI installer and shared-storage images,
  plus an evidence-gated stable bundle for ROCKPro64 Full Phase 0 support.

## Non-goals

- No stable hardware support claim is made until serial validation evidence is
  imported under `validation/rockpro64/stable/`.
- No claim is made for NVMe or any unvalidated storage/controller path.

## Artifact Contract

- `rockpro64-uboot`: board-oriented U-Boot sources, config fragments, and real
  upstream build outputs.
- `rockpro64-spi-firmware`: SPI firmware bundle, Rockchip payload binaries, and
  layout notes.
- `rockpro64-spi-installer-img`: SD installer image producing
  `spi.installer.img`.
- `rockpro64-spi-installer-img-experimental`: the same bootable installer image
  exposed through an experimental flashable bundle.
- `rockpro64-shared-disk-image-img`: shared-storage image producing
  `shared.disk-image.img`.
- `rockpro64-release-bundle`: release-candidate bundle with `release.json`,
  `sha256sums.txt`, and provenance.
- `rockpro64-stable-release-bundle`: stable promotion bundle that fails without
  imported hardware evidence.

## Release Rule

If a future release expands stable hardware support, it must add matching
validation logs and update the stable manifest claims only for paths that
passed on hardware.
