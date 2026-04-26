# Tow-Boot Replacement Gap for ROCKPro64

This document tracks what `bootswain` is missing before it can replace
Tow-Boot for the PINE64 ROCKPro64 firmware role.

The scope is intentionally ROCKPro64-specific. A full Tow-Boot replacement here
means replacing the ROCKPro64 on-device firmware experience: building the
firmware, installing it to SPI or shared storage, finding and booting installed
systems, presenting useful recovery interfaces, and validating releases on
hardware.

Today `bootswain` is host-side tooling only. It can inspect image artifacts,
flash SD media, and run serial-based probe workflows against stock U-Boot. It
does not yet build, install, configure, boot, update, or maintain on-device
firmware.

## Phase 0 Docs

These docs define the first implementation contract and release procedure for a
ROCKPro64 Tow-Boot replacement:

- [ROCKPro64 Tow-Boot Phase 0 contract](rockpro64-towboot-phase0-contract.md)
- [ROCKPro64 Tow-Boot Phase 0 release guide](rockpro64-towboot-phase0-release.md)

## Current State

Covered by the repository today:

- Image inspection for `.img` and `.img.zst`, including size, compression, and
  SHA-256 metadata.
- Linux host-side SD flashing with target-device validation, mounted-disk
  rejection, dry-run support, raw-copy and zstd-decompress strategies, and JSON
  output.
- ROCKPro64 serial probing around stock U-Boot, currently focused on a USB
  sequence: wait for prompt, `usb start`, `usb tree`, `usb reset`, `usb tree`.
- Probe output as raw serial logs plus machine-readable `trial.json` and
  `summary.json`.
- Reproducible release-candidate RK3399 U-Boot and TF-A packaging through Nix.
- SD-bootable SPI installer and shared-storage raw images with canonical
  `spi.installer.img` and `shared.disk-image.img` names.
- U-Boot boot menu fragments for installer and installed-firmware roles.
- Evidence-gated stable promotion through `validation/rockpro64/stable/`.

Not covered today:

- Stable hardware claims without imported ROCKPro64 serial logs.
- Persistent firmware environment edits as a supported workflow.
- NVMe boot as a release claim.

## Replacement Target

Tow-Boot's ROCKPro64 target is the baseline being replaced, not Tow-Boot's full
multi-board distribution.

For ROCKPro64, Tow-Boot documents:

- Device identifier `pine64-rockpro64`.
- SoC `rockchip-rk3399`.
- Best-effort support level.
- Dedicated firmware storage support.
- Recommended installation to SPI.
- Shared-storage installation by writing `shared.disk-image.img` to SD or eMMC.

The relevant behavior target is a boring firmware experience:

- Firmware is independent from the installed operating system when installed to
  dedicated storage.
- Installation and configuration are menu-driven where possible.
- Generic ARM distributions can boot by UEFI, preferably, or by
  extlinux-compatible U-Boot flows.
- Bootable targets are discoverable and listed.
- Serial remains available for diagnostics and advanced recovery.

## Gap Matrix

| Area | Status | Current `bootswain` state | Needed for Tow-Boot replacement |
| --- | --- | --- | --- |
| Host image inspection | `covered` | Computes image metadata and SHA-256 for `.img` and `.img.zst`. | Keep as release and installer validation support. |
| Host SD flashing | `partial` | Can flash supplied images to whole-disk targets safely from Linux. | Add artifact-specific guardrails, target sizing checks, post-write verification, and operator docs for installer media. |
| Serial probing | `partial` | Runs repeatable ROCKPro64 USB probe trials against stock U-Boot. | Expand into a hardware validation suite for install, boot, recovery, and storage scenarios. |
| Firmware build artifacts | `covered for RC` | Builds U-Boot, TF-A, SPI installer, shared image, checksums, release metadata, and provenance. | Import hardware evidence before stable promotion. |
| RK3399 packaging | `covered for RC` | Generates `idbloader.img`, `u-boot.itb`, `u-boot-rockchip.bin`, and `u-boot-rockchip-spi.bin`. | Validate on hardware before stable claims. |
| SPI installer | `partial` | Builds an SD installer with menu actions to flash and erase SPI. | Complete hardware validation and archive serial logs. |
| Safe on-device flashing | `partial` | Adds board check, SPI probe, payload bounds, write, readback, compare, and erase status checks. | Confirm failure modes on hardware. |
| Shared-storage image | `partial` | Builds a raw shared-storage image with Rockchip loader and a firmware FAT partition. | Validate SD/eMMC boot before marking flashable in stable manifests. |
| Boot discovery | `partial` | Configures standard boot scanning for eMMC, SD, USB, and NVMe menu entries. | Claim only paths with hardware logs. |
| UEFI boot | `partial` | Enables U-Boot EFI loader and bootmeth support. | Validate real UEFI installers for each claimed storage class. |
| extlinux boot | `partial` | Enables extlinux bootmeth support. | Validate real extlinux images for each claimed storage class. |
| Boot order policy | `covered for RC` | Release metadata and firmware env use eMMC, SD, USB, NVMe order. | Keep claims aligned with validation evidence. |
| Firmware environment | `covered for RC` | Persistent environment remains disabled and documented. | Revisit only with a reviewed storage policy. |
| Boot menu and UI | `partial` | Provides serial boot menu, boot target entries, SPI flash/erase in installer, and shell escape. | Validate local input paths only if claimed. |
| Serial baseline | `partial` | Probe tooling defaults to 115200 baud and documents the ROCKPro64 UART caveat. | Ensure firmware itself uses a documented 115200 serial baseline and supports installer and recovery operation over serial. |
| Release process | `partial` | Produces release-candidate bundles and a stable bundle that requires imported evidence. | Add changelog/release notes and real validation logs. |
| Phone/tablet shortcuts | `not RockPro64 scope` | Not implemented. | Volume-button boot modes and USB mass-storage gadget mode are not ROCKPro64 acceptance criteria unless project scope expands. |

## Detailed Missing Features

### Firmware Build Artifacts

`bootswain` needs a reproducible ROCKPro64 firmware build pipeline. The output
must be more than a host-side utility package: it must produce the binaries and
disk images a user can actually install.

Required outputs:

- ROCKPro64 U-Boot build from a pinned source and configuration.
- RK3399 TPL/SPL output suitable for Boot ROM loading.
- TF-A BL31 integration.
- U-Boot proper packaged as the board expects.
- SPI-installable firmware image.
- SD/eMMC shared-storage disk image.
- SD-bootable SPI installer image.
- SHA-256 checksums and release metadata for all published artifacts.

Acceptance criteria:

- A clean checkout can build all ROCKPro64 artifacts without manual patching.
- Artifact names clearly distinguish installer, SPI firmware, and shared-storage
  images.
- Release metadata records U-Boot, TF-A, configuration, and source revisions.

### On-Device Installer

Tow-Boot's ROCKPro64 flow expects the user to boot an installer image and choose
a menu action that flashes firmware to SPI. `bootswain` has host-side flashing
only, so the on-device side is missing.

Required installer behavior:

- Boot from SD on ROCKPro64 even when the target installation is SPI.
- Present a firmware installer menu.
- Confirm the detected board identity before writing.
- Probe the SPI flash and report useful failures.
- Read the new firmware payload from installer media.
- Use a safe write strategy that avoids leaving a half-written SPI image first
  in the RK3399 boot chain when recovery from SD/eMMC remains possible.
- Offer a complete SPI erase/uninstall action.
- Reboot or return to menu after success, with clear operator messaging.

Acceptance criteria:

- Fresh SPI installation works from SD media.
- Reinstalling over an existing firmware works.
- Erasing SPI returns the board to the next RK3399 boot source.
- Power-loss and failed-write behavior is documented and tested.

### Boot Behavior

Replacing Tow-Boot means `bootswain`-built firmware must boot installed systems,
not only expose a U-Boot prompt for probes.

Required boot behavior:

- Standards-based boot discovery using U-Boot standard boot or equivalent distro
  boot configuration.
- UEFI boot support as the preferred generic distribution path.
- extlinux-compatible boot support as a fallback.
- Boot target scanning for ROCKPro64-relevant media: SD, eMMC, USB mass storage,
  and NVMe when the underlying U-Boot build supports it.
- A stable and documented boot order for SPI-installed firmware.
- A documented shared-storage priority policy if shared-storage images are
  supported.
- Useful behavior when no bootable media is present, such as a menu or message
  explaining available recovery paths.

Acceptance criteria:

- At least one UEFI installer boots from each supported storage class that is
  claimed in the release notes.
- At least one extlinux-based distribution image boots from each storage class
  that is claimed in the release notes.
- Failure to find bootable media leaves the board recoverable over serial.

### Firmware Storage and Environment

ROCKPro64 has dedicated SPI flash, and Tow-Boot's board configuration records a
16 MiB SPI size. `bootswain` needs an explicit firmware storage map and
environment policy before it can safely own that storage.

Required policy:

- Document the SPI layout for ROCKPro64 firmware and reserved space.
- Decide whether the SPI variant saves a U-Boot environment.
- Define the environment offset and size if persistent environment is enabled.
- Define noenv behavior for installer and shared-storage images.
- Document how shared-storage firmware protects its partition and what users may
  resize or modify.
- Avoid letting stale saved environments change installer behavior.

Acceptance criteria:

- The firmware storage layout is documented before release.
- Installer images have deterministic behavior independent of stale
  environments.
- User-modified boot settings either persist intentionally or are explicitly
  unsupported.

### User Interface

Tow-Boot's target experience is menu-driven and familiar. A replacement should
avoid requiring routine users to type U-Boot commands.

Required interface behavior:

- Boot menu reachable from serial and any supported local input path.
- Listed bootable targets when firmware can discover them.
- Installer menu actions for flashing and erasing SPI.
- Diagnostic entries for board, firmware version, storage visibility, and boot
  discovery results.
- Escape path to the U-Boot shell for advanced recovery.
- Serial baseline documented and tested at 115200 baud.

Acceptance criteria:

- A user can install firmware, inspect bootable targets, boot a selected target,
  and erase SPI without memorizing U-Boot commands.
- Serial-only operation is sufficient for headless ROCKPro64 recovery.

### Validation and Release Process

Current probe support is a useful starting point, but a Tow-Boot replacement
requires release gates that exercise real firmware behavior on real hardware.

Required validation:

- Cold-boot trials from blank SPI, existing SPI firmware, SD installer media,
  eMMC, USB mass storage, and NVMe.
- Install, reinstall, and erase SPI tests.
- UEFI installer boot tests.
- extlinux image boot tests.
- USB enumeration and reset tests, extending the current probe sequence.
- NVMe discovery and boot tests if NVMe support is claimed.
- eMMC and SD boot tests with and without SPI installed.
- Serial log capture for every hardware trial.
- Published validation summary per release.

Acceptance criteria:

- Release notes list exactly which storage and boot paths were tested.
- Unsupported or untested paths are not advertised as working.
- Failures produce enough logs to reproduce or triage the issue.

## Validation Checklist

Before declaring `bootswain` a ROCKPro64 Tow-Boot replacement:

- Build all firmware and installer artifacts from a clean checkout.
- Flash the SD installer image using `bootswain flash sd`.
- Install firmware to SPI from the on-device installer menu.
- Boot a UEFI installer from SD.
- Boot a UEFI installer from USB mass storage.
- Boot a UEFI installation from eMMC.
- Boot a UEFI installation from NVMe if NVMe support is advertised.
- Boot an extlinux-compatible image from SD.
- Boot an extlinux-compatible image from eMMC.
- Confirm no-bootable-media behavior is useful and recoverable.
- Confirm the U-Boot shell is reachable for recovery.
- Erase SPI and verify RK3399 falls through to the next available boot source.
- Reinstall SPI firmware after erase.
- Capture serial logs and machine-readable summaries for all trials.
- Publish checksums, firmware revisions, build inputs, and validation results.

## References

- [Tow-Boot ROCKPro64 device page](https://tow-boot.org/devices/pine64-rockpro64.html)
- [Tow-Boot Getting Started](https://tow-boot.org/getting-started.html)
- [Tow-Boot project goals](https://tow-boot.org/)
- [Tow-Boot differences from U-Boot](https://tow-boot.org/differences-from-u-boot.html)
- [Tow-Boot variants](https://tow-boot.org/variants.html)
- [Tow-Boot firmware storage map](https://tow-boot.org/in-depth/firmware-storage-map.html)
- [Tow-Boot ROCKPro64 board config](https://raw.githubusercontent.com/Tow-Boot/Tow-Boot/released/boards/pine64-rockpro64/default.nix)
- [U-Boot standard boot overview](https://docs.u-boot.org/en/v2025.10/develop/bootstd/overview.html)
- [U-Boot generic distro configuration concept](https://docs.u-boot.org/en/v2024.07/develop/distro.html)
