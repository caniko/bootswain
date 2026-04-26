# ROCKPro64 Firmware Source

This directory is the source of truth for the upstream-first ROCKPro64 Tow-Boot
replacement plan in `bootswain`.

Status:

- real RK3399 U-Boot and TF-A packaging is wired through the flake
- SPI installer and shared-storage images are built as release candidates
- stable claims require imported ROCKPro64 serial validation evidence
- NVMe and any unvalidated boot paths remain unclaimed

Files:

- `contract.md`: board-level firmware contract and scope
- `spi-layout.md`: current SPI storage map used by the installer and firmware
- `../lib/generic-boot-menu.cmd`: shared boot-menu template rendered for
  board firmware scripts
- `u-boot.config.fragment`: U-Boot configuration fragment appended to the
  upstream board defconfig
- `uboot.env`: installer runtime environment defaults and policy notes
- `boot.cmd`: ROCKPro64 installer fragment rendered with the shared menu
  template to produce the installer U-Boot boot script

The flake renders separate installer and installed-firmware roles. The
installer role boots from SD and can flash/erase SPI; the firmware role is used
for the SPI payload and shared-storage image and does not expose installer
write actions. The default release bundle is a release candidate until hardware
evidence is imported.

The text default environment intentionally includes the RK3399 extlinux load
addresses from upstream U-Boot. Supplying `CONFIG_ENV_DEFAULT_ENV_TEXT_FILE`
replaces the implicit board defaults, so these values must stay in sync with
the boot script fallback guards.
