# Status and Support Claims

Bootswain targets the PINE64 ROCKPro64 firmware role. The goal is a
ROCKPro64-specific replacement path for the on-device firmware experience:
building firmware, installing it to SPI or shared storage, finding and booting
installed systems, presenting recovery interfaces, and validating releases on
hardware.

## Covered Today

- Image inspection for `.img` and `.img.zst`, including size, compression, and
  SHA-256 metadata.
- Linux host-side SD flashing with target-device validation, mounted-disk
  rejection, dry-run support, raw-copy and zstd-decompress strategies, and JSON
  output.
- ROCKPro64 serial probing and validation workflows.
- Reproducible release-candidate RK3399 U-Boot and TF-A packaging through Nix.
- SD-bootable SPI installer and shared-storage raw images with canonical
  `spi.installer.img` and `shared.disk-image.img` names.
- U-Boot boot menu fragments for installer and installed-firmware roles.
- Evidence-gated stable promotion through `validation/rockpro64/stable/`.

## Not Claimed

- Stable hardware support without imported ROCKPro64 serial logs.
- Persistent firmware environment edits as a supported workflow.
- NVMe OS boot as a release claim.
- Other boards in the Tow-Boot lineup.
- Volume-button boot modes, USB gadget mode, and network boot.

## Replacement Target

Tow-Boot's ROCKPro64 target is the baseline being replaced, not Tow-Boot's full
multi-board distribution.

For ROCKPro64, the relevant target behavior is:

- firmware is independent from the installed operating system when installed to
  dedicated SPI storage
- installation and configuration are menu-driven where possible
- generic ARM distributions boot by UEFI, preferably, or by extlinux-compatible
  U-Boot flows
- bootable targets are discoverable and listed
- serial remains available for diagnostics and recovery

Release notes may claim only paths with passing hardware logs. Untested paths
remain explicitly unsupported or unclaimed.
