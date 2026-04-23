# ROCKPro64 Tow-Boot Phase 0 Contract

This document defines the first release contract for a ROCKPro64 Tow-Boot
replacement. It is intentionally narrow: the goal is a board-specific firmware
release with explicit artifacts, layout rules, boot order, environment policy,
and validation claims.

## Scope

Phase 0 covers the firmware-facing behavior needed to replace Tow-Boot on the
PINE64 ROCKPro64:

- reproducible firmware artifacts for the board
- a bootable SPI installer image
- a shared-storage image for SD or eMMC
- a documented boot policy and recovery story
- release validation claims that match what was actually tested on hardware

Phase 0 does not claim support for other boards or future Tow-Boot features that
are unrelated to ROCKPro64 booting and installation.

## Artifact Set

Release outputs are expected to use these artifact names:

- `spi.installer.img`
- `shared.disk-image.img`
- `idbloader.img`
- `u-boot.itb`
- `bl31.elf`
- `sha256sums.txt`
- `release.json`

Compressed release assets may additionally be published as `.zst` variants, but
the base names above are the contract.

The end-user images are the installer image and the shared-storage image. The
loader and manifest artifacts are release inputs and validation aids.

## Storage Assumptions

The contract assumes the ROCKPro64 storage model documented by Tow-Boot:

- dedicated SPI flash is present and is the recommended installation target
- the board uses the Rockchip RK3399 boot chain
- the SoC startup order prefers SPI, then eMMC, then SD
- the dedicated SPI flash size is treated as 16 MiB
- shared-storage installs are written as a discrete image to SD or eMMC

The SPI install path owns the whole flash device. The shared-storage path owns a
protected firmware partition or equivalent reserved span on the target medium.
The operating system must not be expected to manage that protected region.

## Boot Order

The firmware boot policy is:

1. try the board-recognized internal boot sources in the Rockchip order
2. scan installed OS targets in the documented firmware order
3. prefer ordinary distro boot discovery before dropping to a shell
4. keep serial recovery available whenever boot discovery fails

For the firmware scan order itself, the release should present the following
media classes in a stable order:

1. eMMC
2. SD
3. USB mass storage
4. NVMe, if the build and board support it

Standard boot or distro boot is the preferred discovery mechanism. UEFI boot is
the preferred generic distro path, with extlinux as the fallback.

## Environment Policy

Phase 0 does not rely on user-customized persistent environment state.

- The installer image must ignore stale saved environment data.
- A fresh install must boot correctly with factory defaults.
- If persistent environment is enabled in the SPI image, it must live in a
  reserved tail region and stay outside the boot payload.
- If persistent environment is not enabled, the release must say so plainly and
  the firmware must behave as if `env default -a` were the recovery baseline.
- Shared-storage images must not allow ordinary OS activity to corrupt the
  firmware boot policy through a writable environment block.

The release policy is conservative: deterministic booting matters more than
preserving ad hoc environment edits.

## Validation Claims

A Phase 0 release may only claim paths that were actually exercised on hardware.
The release notes must name the tested combinations explicitly.

At minimum, the release must distinguish between:

- build success from a clean checkout
- SD installer boot
- SPI installation
- SPI reinstall
- SPI erase or uninstall
- SD boot from shared storage
- eMMC boot from shared storage
- UEFI boot, if claimed
- extlinux boot, if claimed
- serial recovery

Any path not validated in the release notes is unclaimed, even if it is
plausible or expected.

## Unsupported Paths

Phase 0 does not claim:

- volume-button boot modes
- USB gadget or mass-storage device mode
- network boot
- other boards in the Tow-Boot lineup
- arbitrary repartitioning of the protected firmware area
- custom persistent environment edits as a supported workflow
- untested storage classes or controller combinations

## Build and Install Contract

The build contract is:

- the release is built from a clean checkout
- all board-specific build inputs are pinned or recorded in release metadata
- checksums and source revisions are published with the release artifacts

The install contract is:

- write `spi.installer.img` to SD media
- boot the installer on the ROCKPro64
- choose the installer action that flashes firmware to SPI
- remove installation media and verify the board boots from SPI
- write `shared.disk-image.img` to SD or eMMC when using the shared-storage
  path

The release notes must not ask operators to guess which image is which. The
artifact name itself should tell them whether it is an installer, a SPI target,
or a shared-storage image.

## References

- [Tow-Boot ROCKPro64 device page](https://tow-boot.org/devices/pine64-rockpro64.html)
- [Tow-Boot firmware storage map](https://tow-boot.org/in-depth/firmware-storage-map.html)
- [Tow-Boot ROCKPro64 board config](https://raw.githubusercontent.com/Tow-Boot/Tow-Boot/released/boards/pine64-rockpro64/default.nix)
- [U-Boot standard boot overview](https://docs.u-boot.org/en/v2025.10/develop/bootstd/overview.html)
- [U-Boot generic distro configuration concept](https://docs.u-boot.org/en/v2024.07/develop/distro.html)
