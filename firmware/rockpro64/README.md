# ROCKPro64 Firmware Scaffold

This directory is the source of truth for the upstream-first ROCKPro64 Tow-Boot
replacement plan in `bootswain`.

Status:

- scaffold only
- no hardware validation claimed
- source files here are consumed by the flake package outputs

Files:

- `contract.md`: board-level firmware contract and scope
- `spi-layout.md`: provisional SPI storage map
- `u-boot.config.fragment`: U-Boot configuration fragment
- `uboot.env`: U-Boot environment fragment
- `boot.cmd`: U-Boot script fragment

The flake packages copy or package these files into the named firmware outputs.
