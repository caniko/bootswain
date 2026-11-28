# Pi 5 and ROCK 5A UEFI artifacts

BootsWain packages Pi 5 U-Boot **v2026.07**, Raspberry Pi firmware/DTBs
**1.20260915**, and ROCK 5A EDK2 **v1.1**, all with fixed source hashes. Pi 5,
ROCK 5A, Pi 3 B+, and ROCKPro64 have distinct board identities.

| Output | Contents |
| --- | --- |
| `.#pi5-uboot` | ARM64 U-Boot binary and the build's complete `.config` |
| `.#pi5-uefi-boot-files` | `config.txt`, U-Boot, C1/D0 DTBs, overlays, license |
| `.#pi5-boot-partition-image` | 512 MiB FAT filesystem containing those boot files |
| `.#pi5-release-bundle` | Image, typed release manifest, sizes and SHA-256 checksums |
| `.#rock5a-uefi-firmware` | Pinned upstream ROCK 5A UEFI image |
| `.#rock5a-release-bundle` | Image, typed release manifest, sizes and SHA-256 checksums |

The Pi build asserts `EFI_LOADER`, `EFI_VARIABLE_FILE_STORE`, `FAT_WRITE`,
`FAT_RENAME`, and `BOOTMETH_EFILOADER` in its final configuration. The
`uefi-artifacts` Nix check validates the images, both silicon revisions, manifest
bounds, and checksums. Rust tests exercise board names and read-only probing.
These checks verify packaging. EFI handoff on hardware remains unmeasured.

## Inspection and initial lab installation

Inspect the bundle before flashing with BootsWain:

```sh
bootswain firmware inspect --manifest /path/to/bundle/release.json --json
bootswain firmware artifacts --manifest /path/to/bundle/release.json --json
```

The `.#flash` targets `pi5-boot-partition` and `rock5a-uefi` are release
candidates marked `flashable: false`; use `--allow-non-flashable` only in the
lab. The Pi image is a standalone FAT filesystem, not a partitioned OS
installer. Flash it only to dedicated, unmounted firmware media; install the
OS/ESP separately. The raw flasher still checks the whole device and verifies
bytes. Do not use it to update a firmware partition on an occupied disk.
Neither wrapper writes to live SPI/EEPROM or activates firmware automatically.

The Pi EEPROM must already support the board/media setup. Record its version
and keep compatible rescue media. Pi 5 boots without Pi 3's `start.elf` on the
FAT filesystem.

The upstream ROCK 5A image supplies Rockchip startup firmware and EDK2. Use
**Device Tree / Mainline** Linux mode. Preserve the upstream reserved firmware
region; place OS partitions at or beyond 16 MiB. The variable region
`0x7c0000..0x7effff` is excluded from the firmware image and must survive later
updates. Do not use whole-device reimaging to upgrade a disk with existing data.

## Post-handoff evidence

On the running Linux target:

```sh
bootswain probe platform --expected-board pi5
bootswain probe platform --expected-board rock5a
```

The JSON result binds the expected board to the device-tree compatible strings
and records kernel release and EFI/efivars visibility. Runtime variable writes,
boot counting, watchdog reset, and external rescue remain `not-measured`.
`--sys-root` and `--proc-root` support captured fixtures, not physical
qualification. Pi 5 probing rejects Compute Module 5.

U-Boot's file-backed EFI variables persist firmware-time writes, **not** Linux
runtime `SetVariable` writes. An `efivars` directory alone cannot prove runtime
persistence. Downstream systemd-boot counting needs readiness-bound file
blessing and boot selection that works without runtime variable writes. Verify
ROCK 5A's shared SD/eMMC variable access on the actual setup.

Before qualifying a target, record EFI handoff to the intended kernel,
ESP/storage layout, boot-count exhaustion and fallback, readiness blessing,
watchdog handoff, interrupted installation/update and power cuts, and external
rescue boot with state retention. Recovery before firmware/networking works
requires independent power/serial access. The consumer must coordinate
application and OS trials.

## Sources

- [U-Boot Pi board implementation](https://github.com/u-boot/u-boot/blob/v2026.07/board/raspberrypi/rpi/rpi.c)
- [U-Boot EFI variable backend contract](https://github.com/u-boot/u-boot/blob/v2026.07/lib/efi_loader/Kconfig)
- [U-Boot boot method configuration](https://github.com/u-boot/u-boot/blob/v2026.07/boot/Kconfig)
- [Raspberry Pi firmware release](https://github.com/raspberrypi/firmware/releases/tag/1.20260915)
- [EDK2 RK3588 v1.1 installation and flash layout](https://github.com/edk2-porting/edk2-rk3588/tree/v1.1)
