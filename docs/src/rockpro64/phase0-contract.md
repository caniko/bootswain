# ROCKPro64 Phase 0 Contract

This first release contract for a ROCKPro64 Tow-Boot replacement defines the
board's firmware artifacts, layout, boot order, environment, and validation claims.

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

- `spi.installer.img`
- `shared.disk-image.img`
- `idbloader.img`
- `u-boot.itb`
- `bl31.elf`
- `sha256sums.txt`
- `release.json`

Releases may also publish `.zst` variants; the base names above are the
contract. The installer and shared-storage images are for end users.

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

For generic distros, prefer UEFI and fall back to extlinux.
On the tested ROCKPro64 v2.1, U-Boot enumerates eMMC as `mmc0` and SD as
`mmc1`. See [Operator Notes](./operator-notes.md) for the serial checks used to
verify the mapping.

## Environment Policy

Phase 0 does not rely on user-customized persistent environment state.
Deterministic boot takes priority over ad hoc environment edits.

- The installer image must ignore stale saved environment data.
- A fresh install must boot correctly with factory defaults.
- Phase 0 disables the persistent environment.
- Shared-storage images must prevent ordinary OS activity from corrupting
  firmware boot policy through a writable environment block.

## Validation Claims

Claim only paths tested on hardware. Name the exact combinations in the
release notes.

Distinguish at least these cases:

- build success from a clean checkout
- SD installer boot
- SPI installation, reinstall, and erase
- SD and eMMC boot from shared storage
- UEFI and extlinux boot paths, if claimed
- serial recovery

Paths not validated in the release notes are unclaimed, even if plausible.
