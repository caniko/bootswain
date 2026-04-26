# ROCKPro64 Phase 0 Contract

This is the first release contract for a ROCKPro64 Tow-Boot replacement. It is
intentionally narrow: the goal is a board-specific firmware release with
explicit artifacts, layout rules, boot order, environment policy, and validation
claims.

## Scope

Phase 0 covers firmware-facing behavior for the PINE64 ROCKPro64:

- reproducible firmware artifacts for the board
- a bootable SPI installer image
- a shared-storage image for SD or eMMC
- a documented boot policy and recovery story
- release validation claims that match what was tested on hardware

Phase 0 does not claim support for other boards or future Tow-Boot features
unrelated to ROCKPro64 booting and installation.

## Artifact Set

Release outputs use these artifact names:

- `spi.installer.img`
- `shared.disk-image.img`
- `idbloader.img`
- `u-boot.itb`
- `bl31.elf`
- `sha256sums.txt`
- `release.json`

Compressed release assets may additionally be published as `.zst` variants, but
the base names above are the contract. The end-user images are the installer
image and the shared-storage image.

## Storage and Boot Policy

The contract assumes:

- dedicated SPI flash is present and is the recommended installation target
- the board uses the Rockchip RK3399 boot chain
- the SoC startup order prefers SPI, then eMMC, then SD
- the dedicated SPI flash size is treated as 16 MiB
- shared-storage installs are written as a discrete image to SD or eMMC

The firmware scan order is:

1. eMMC
2. SD
3. USB mass storage
4. NVMe, if the build and board support it

UEFI boot is the preferred generic distro path, with extlinux as the fallback.
On the tested ROCKPro64 v2.1, U-Boot enumerates eMMC as `mmc0` and SD as
`mmc1`. See [Operator Notes](./operator-notes.md) for the serial checks used to
verify the mapping.

## Environment Policy

Phase 0 does not rely on user-customized persistent environment state.

- The installer image must ignore stale saved environment data.
- A fresh install must boot correctly with factory defaults.
- Persistent environment is disabled for this milestone.
- Shared-storage images must not allow ordinary OS activity to corrupt firmware
  boot policy through a writable environment block.

Deterministic booting matters more than preserving ad hoc environment edits.

## Validation Claims

A Phase 0 release may only claim paths that were exercised on hardware. Release
notes must name the tested combinations explicitly.

At minimum, the release must distinguish between:

- build success from a clean checkout
- SD installer boot
- SPI installation, reinstall, and erase
- SD and eMMC boot from shared storage
- UEFI and extlinux boot paths, if claimed
- serial recovery

Any path not validated in the release notes is unclaimed, even if it is
plausible.
