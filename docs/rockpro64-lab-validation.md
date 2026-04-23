# ROCKPro64 Lab Validation

ROCKPro64 acceptance is hardware-only. Generic ARM64 QEMU validation can catch
host tooling and U-Boot scripting regressions, but it cannot prove RK3399 boot
ROM behavior, TPL/SPL handoff, TF-A BL31 integration, SPI flash layout, PCIe
enumeration, eMMC behavior, USB power/reset behavior, or NVMe boot reliability.

## Fixture

The hardware validation fixture is:

```sh
validation/rockpro64/lab-plan.json
```

It is intentionally marked with executor `rockpro64-serial`. Until a real lab
run captures serial logs, `bootswain validate run` should materialize this plan
as `not-run`.

Run one non-destructive scenario:

```sh
bootswain validate rockpro64-serial \
  --plan validation/rockpro64/lab-plan.json \
  --port /dev/ttyUSB0 \
  --scenario recovery-console \
  --out bootswain-rockpro64-serial-output
```

Run destructive SPI scenarios only when the board is prepared for recovery:

```sh
bootswain validate rockpro64-serial \
  --plan validation/rockpro64/lab-plan.json \
  --port /dev/ttyUSB0 \
  --scenario spi-install \
  --allow-destructive-spi \
  --out bootswain-rockpro64-serial-output
```

Without `--allow-destructive-spi`, `spi-install` and `spi-erase` scenarios are
reported as `skipped`.

## Hardware Setup

- Board: PINE64 ROCKPro64
- Serial console: 115200 baud
- Required media: SPI installer SD card, UEFI test image, extlinux test image
- Storage classes to claim only after testing: SD, eMMC, USB mass storage, NVMe
- Required output directory: one timestamped directory per full lab run

## Operator Flow

1. Flash installer media with `bootswain flash sd --manifest <manifest> --artifact spi-installer --verify`.
2. Boot the ROCKPro64 from SD and capture the full serial log.
3. Confirm the installer menu identifies ROCKPro64 before any SPI write.
4. Run SPI install and capture success or failure text.
5. Power off, remove SD, boot from SPI, and confirm the U-Boot prompt or boot menu appears.
6. Reinsert installer media and repeat SPI install to validate reinstall behavior.
7. Run full SPI erase and confirm the board returns to a recoverable SD-boot state.
8. Test each advertised boot path with both UEFI and extlinux images.
9. Test no-bootable-media behavior and recovery-console access.
10. Archive `validation-run.json`, serial logs, artifact manifest, and exact image checksums.

## Abort Conditions

Stop the run and do not publish support claims if any of these occur:

- The installer cannot identify the board as ROCKPro64.
- `sf probe` fails or reports an unexpected flash size.
- SPI erase or write fails before verification.
- The board cannot boot from SD after SPI erase.
- Any advertised storage class lacks a captured passing serial log.
- Any boot protocol is claimed without a corresponding UEFI or extlinux test log.

## Release Claim Rule

Release notes may claim only paths with passing hardware logs. Untested paths
must remain explicitly unsupported or unclaimed, even if the equivalent generic
QEMU scenario passes.
