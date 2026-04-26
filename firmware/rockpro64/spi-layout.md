# ROCKPro64 SPI Layout

This document records the layout used by the ROCKPro64 SPI payload artifacts
built in the flake. The default bundle is a release candidate; stable claims
require imported hardware validation evidence.

## Current Map

- `0x00000000` - `0x0002ffff`: `idbloader.img`
- `0x00030000` - `0x0005ffff`: reserved padding
- `0x00060000` - `0x00187fff`: `u-boot.itb`
- `0x00188000` - `0x00ffffff`: reserved and left untouched by the combined SPI
  payload image

## Policy Notes

- The combined `u-boot-rockchip-spi.bin` image is written at offset `0x0` of
  SPI flash.
- The SD installer writes that combined image with `sf update`, and erases the
  full 16 MiB SPI device for uninstall testing.
- Persistent environment is disabled for this milestone. The reserved tail of
  SPI remains unclaimed until the project has a reviewed environment policy and
  hardware promotion logs.
