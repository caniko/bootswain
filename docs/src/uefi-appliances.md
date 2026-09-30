# Pi 5 and ROCK 5A UEFI artifacts

BootsWain packages Pi 5 U-Boot **v2026.07**, Raspberry Pi firmware/DTBs
**1.20260915**, and ROCK 5A EDK2 **v1.1** with fixed source hashes. Both board
identities are distinct from Pi 3 B+ and ROCKPro64.

| Output | Contents |
| --- | --- |
| `.#pi5-uboot` | ARM64 U-Boot binary and the build's complete `.config` |
| `.#pi5-uefi-boot-files` | `config.txt`, U-Boot, C1/D0 DTBs, overlays, license |
| `.#pi5-boot-partition-image` | 512 MiB FAT filesystem containing those boot files |
| `.#pi5-release-bundle` | Image, typed release manifest, sizes and SHA-256 checksums |
| `.#rock5a-uefi-firmware` | Pinned upstream ROCK 5A UEFI image |
| `.#rock5a-release-bundle` | Image, typed release manifest, sizes and SHA-256 checksums |

The Pi build asserts `EFI_LOADER`, `EFI_VARIABLE_FILE_STORE`, `FAT_WRITE`,
`FAT_RENAME`, and `BOOTMETH_EFILOADER` in the resulting configuration. The
`uefi-artifacts` Nix check validates the images, both silicon revisions, manifest
bounds, and checksums. Rust tests exercise board names and read-only probing.
This evidence verifies packaging; actual board EFI handoff remains unmeasured.

## Inspection and initial lab installation

Inspect the bundle before using the ordinary BootsWain flasher:

```sh
bootswain firmware inspect --manifest /path/to/bundle/release.json --json
bootswain firmware artifacts --manifest /path/to/bundle/release.json --json
```

The `.#flash` wrapper exposes `pi5-boot-partition` and `rock5a-uefi`. These
release-candidate artifacts retain `flashable: false`; explicit lab use selects
`--allow-non-flashable`. The Pi image is a standalone FAT filesystem image,
not a partitioned OS installer. Flash it only onto dedicated, unmounted firmware
media; install the OS/ESP separately. The raw flasher's existing whole-device
checks and byte verification remain in force. It is unsuitable for updating an
occupied disk's firmware partition. Neither wrapper mutates a live SPI/EEPROM
device or performs automatic firmware activation.

The Pi EEPROM must already support the board/media setup. Keep a recorded EEPROM
version and compatible rescue media. Booting Pi 5 does not require Pi 3's
`start.elf` on the FAT filesystem.

For ROCK 5A, the upstream image provides Rockchip startup firmware and EDK2. Use
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

The JSON result binds the expected board to the device-tree compatible strings,
records kernel release and EFI/efivars visibility, and explicitly leaves runtime
variable writes, boot counting, watchdog reset, and external rescue as
`not-measured`. `--sys-root` and `--proc-root` support captured fixtures; those
fixtures are not physical qualification. Pi 5 probing rejects Compute Module 5.

U-Boot's file-backed EFI variables persist writes made during firmware execution.
It does **not** provide persistent runtime `SetVariable` writes from Linux; an
`efivars` directory alone cannot prove that capability. A downstream counted
systemd-boot design must support readiness-bound file blessing and a selection
mechanism that does not depend on unavailable runtime variable writes. ROCK 5A's
shared SD/eMMC variable access also needs verification on the actual arrangement.

Before target qualification, record EFI handoff to the intended kernel, ESP/storage
layout, boot-count exhaustion and fallback, readiness blessing, watchdog handoff,
interrupted installation/update and power cuts, and external rescue boot with
state retention. Recovery before functioning firmware/networking needs independent
power/serial access. Application and OS trials must be coordinated by the consumer.

## Sources

- [U-Boot Pi board implementation](https://github.com/u-boot/u-boot/blob/v2026.07/board/raspberrypi/rpi/rpi.c)
- [U-Boot EFI variable backend contract](https://github.com/u-boot/u-boot/blob/v2026.07/lib/efi_loader/Kconfig)
- [U-Boot boot method configuration](https://github.com/u-boot/u-boot/blob/v2026.07/boot/Kconfig)
- [Raspberry Pi firmware release](https://github.com/raspberrypi/firmware/releases/tag/1.20260915)
- [EDK2 RK3588 v1.1 installation and flash layout](https://github.com/edk2-porting/edk2-rk3588/tree/v1.1)
