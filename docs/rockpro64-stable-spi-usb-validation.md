# ROCKPro64 Stable Validation Evidence

This record defines the evidence required before a ROCKPro64 release candidate
may be promoted to stable. The default generated bundle is not stable; it emits
`tested: false` claims until hardware logs are imported.

- SPI installer menu from SD media
- SPI install, reinstall, and full erase recovery
- SPI post-install U-Boot prompt or boot menu
- shared-storage boot from SD and eMMC
- USB boot through UEFI and extlinux
- no-bootable-media behavior and serial recovery

The stable release does not claim NVMe OS boot, network boot, USB gadget mode,
persistent environment edits, or support for other boards unless matching logs
are added first.

## Required Release Evidence

Archive these files with the release artifacts:

- `validation/rockpro64/stable/validation-run.json` from the ROCKPro64 serial
  validation run
- full serial logs for every claimed scenario
- release `release.json`
- generated `release-notes.md`
- `sha256sums.txt`
- exact stable installer image checksum

The stable promotion derivation `.#rockpro64-stable-release-bundle` fails until
the evidence directory contains `validation-run.json`, every claimed scenario is
present with status `passed`, and every referenced serial log exists. If a
future release expands the claim to NVMe OS boot, add passing hardware logs
before updating the manifest claims.
