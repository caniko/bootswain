# ROCKPro64 SPI Layout, Provisional

This is a scaffolded storage map for a 16 MiB SPI device. The offsets below
must be revalidated against upstream U-Boot and board documentation before any
hardware release.

## Provisional Map

- `0x00000000` - `0x0007ffff`: `idbloader` scaffold
- `0x00080000` - `0x003fffff`: `u-boot.itb` scaffold
- `0x00400000` - `0x00ffffff`: reserved for future environment or recovery use

## Policy Notes

- Keep installer images deterministic with no dependency on stale persistent
  environment data.
- Do not write user data into the reserved region without a reviewed policy.
- Do not treat this map as hardware-validated.
