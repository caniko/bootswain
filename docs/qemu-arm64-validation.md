# Generic ARM64 QEMU Validation

`bootswain validate qemu-arm64` is a host-side smoke runner for generic ARM64
U-Boot behavior under QEMU `virt`. It is not a ROCKPro64 emulator and it must
not be used as evidence for RK3399 SPL/TPL/TF-A, SPI flash, eMMC wiring, PCIe,
or board-specific boot behavior.

Local baseline used for this scaffold:

- QEMU binary: `qemu-system-aarch64`
- Machine: `virt`
- Useful synthetic storage devices: virtio block, USB storage, and NVMe
- Unsupported by QEMU in this workflow: `rockpro64` or `rk3399` machine models

## Fixture

The generic validation fixture is:

```sh
validation/qemu-arm64/generic-smoke.json
```

The CI-safe boot fixture is:

```sh
validation/qemu-arm64/boot-smoke.json
```

`boot-smoke.json` only waits for a U-Boot prompt and is used by the Nix
`qemu-arm64-boot-smoke` check. `generic-smoke.json` covers prompt detection,
boot menu scripting, `bootflow scan`, and synthetic storage discovery for manual
QEMU exercises.

## Dry Run

Use dry-run mode to verify command construction and JSON output without starting
QEMU:

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

Each scenario is reported as `not-run`.

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

The flake check runs the real prompt smoke automatically:

```sh
nix flake check
```

## Release Claim Rule

Passing QEMU validation means only that generic ARM64 U-Boot command handling,
serial log capture, timeout handling, and `bootswain` validation reporting work.
ROCKPro64 support still requires a hardware validation run using the
ROCKPro64 lab plan.
