# Generic ARM64 QEMU Validation

`bootswain validate qemu-arm64` smoke-tests generic ARM64 U-Boot from the host
under QEMU `virt`. It does not emulate ROCKPro64. Do not use it as evidence for
RK3399 SPL/TPL/TF-A, SPI flash, eMMC wiring, PCIe, or board-specific boot behavior.

Local baseline:

- QEMU binary: `qemu-system-aarch64`
- Machine: `virt`
- Synthetic storage: virtio block, USB storage, and NVMe
- Machine models unsupported by this workflow: `rockpro64` and `rk3399`

## Fixtures

Generic validation:

```sh
validation/qemu-arm64/generic-smoke.json
```

CI-safe boot:

```sh
validation/qemu-arm64/boot-smoke.json
```

The Nix `qemu-arm64-boot-smoke` check uses `boot-smoke.json` to wait only for a
U-Boot prompt. For manual QEMU runs, `generic-smoke.json` covers prompt detection,
boot menu scripting, `bootflow scan`, and synthetic storage discovery.

## Dry Run

Check command construction and JSON output without starting QEMU:

```sh
bootswain validate qemu-arm64 \
  --plan validation/qemu-arm64/generic-smoke.json \
  --u-boot ./u-boot.bin \
  --out bootswain-qemu-arm64-output \
  --dry-run \
  --json
```

Dry run writes:

- `validation-run.json`
- `qemu-command.json`
- `serial.log`
- `qemu-stderr.log`

Dry run reports each scenario as `not-run`.

## Execution

Run with a generic ARM64 U-Boot binary built for QEMU `virt`:

```sh
bootswain validate qemu-arm64 \
  --plan validation/qemu-arm64/boot-smoke.json \
  --u-boot "$(nix build --no-link --print-out-paths .#qemu-arm64-uboot)/u-boot.bin" \
  --out bootswain-qemu-arm64-output \
  --timeout-secs 20
```

Attach a synthetic disk when exercising block discovery:

```sh
bootswain validate qemu-arm64 \
  --plan validation/qemu-arm64/generic-smoke.json \
  --u-boot ./u-boot.bin \
  --disk ./disk.img \
  --disk-interface virtio \
  --out bootswain-qemu-arm64-output
```

Supported `--disk-interface` values are `virtio`, `usb-storage`, and `nvme`.

A QEMU pass covers only generic ARM64 U-Boot command handling, serial log
capture, timeout handling, and Bootswain validation reporting. ROCKPro64
support still requires a hardware validation run.
