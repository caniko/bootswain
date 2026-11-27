# Bootswain

<!-- simit:badges:start -->

[![CI](https://img.shields.io/badge/CI-drift-2088ff)](https://github.com/caniko/fleet/actions) [![docs](https://img.shields.io/badge/docs-enabled-6f42c1)](docs)

<!-- simit:badges:end -->

`bootswain` is a Rust workspace for repeatable host-side flashing, firmware
inspection, serial probing, and U-Boot validation. It focuses on ROCKPro64
firmware, with hardware-validated Raspberry Pi 3 B+ SD card firmware (stable
evidence under `validation/raspberrypi3bplus/stable/`) and best-effort Pi 3
family firmware.

Bootswain also offers Pi 5 U-Boot UEFI and ROCK 5A EDK2 release candidates with
a board-bound, read-only Linux platform probe. See
[UEFI appliances](docs/src/uefi-appliances.md) for outputs, installation limits,
firmware versions, and remaining hardware checks.

Release candidates are easy to build and inspect. Stable support requires real
validation data under `validation/<board>/stable/`.

## What It Does

- Inspect raw and zstd-compressed image artifacts.
- Flash SD or removable block devices with dry-run and verification modes.
- Run reproducible ROCKPro64 USB and serial validation flows.
- Package ROCKPro64 RK3399 U-Boot, TF-A, SPI installer, and shared-storage
  images with Nix.
- Package a Raspberry Pi 3 B+ MBR SD card image (FAT boot partition with Raspberry Pi
  firmware, Pi 3 family DTBs, and U-Boot) with Nix.
- Expose NixOS modules for board boot-file installation and bootable profiles.

## Status

ROCKPro64 is the primary target. The SPI installer is a flashable release
candidate. To flash a shared-storage image for lab work, explicitly allow
non-flashable artifacts.

The Raspberry Pi 3 B+ MBR SD card image is a release candidate. Stable support
requires importing validation evidence under `validation/raspberrypi3bplus/stable/`.

## Quick Start

Enter the development shell:

```sh
nix develop
```

Build and test the workspace:

```sh
cargo build --workspace
cargo test --workspace
```

Run the flake checks:

```sh
nix flake check
```

List Nix-built flash targets:

```sh
nix run .#flash -- --list-targets
```

Dry-run a ROCKPro64 SPI installer flash:

```sh
nix run .#flash -- \
  --target rockpro64-spi-installer \
  --device /dev/sdX \
  --dry-run
```

Dry-run a Raspberry Pi 3 B+ SD card image flash:

```sh
nix run .#flash -- \
  --target raspberrypi3bplus-sd-card \
  --device /dev/sdX \
  --dry-run
```

## CLI Examples

Inspect an image:

```sh
bootswain image inspect --image /path/to/spi.installer.img
```

Inspect an image with JSON output:

```sh
bootswain image inspect \
  --image /path/to/spi.installer.img \
  --json
```

Flash an SD card after reviewing the dry-run output:

```sh
bootswain flash sd \
  --image /path/to/spi.installer.img \
  --device /dev/sdX \
  --verify
```

Run one ROCKPro64 USB probe trial:

```sh
bootswain probe rockpro64-usb \
  --image /path/to/spi.installer.img \
  --port /dev/ttyUSB0 \
  --out ./probe-output
```

Run the generic ARM64 QEMU smoke validator:

```sh
bootswain validate qemu-arm64 \
  --plan validation/qemu-arm64/boot-smoke.json \
  --u-boot /path/to/u-boot.bin \
  --qemu qemu-system-aarch64 \
  --out ./qemu-output
```

## Nix Outputs

Common outputs include:

- `.#bootswain` / `.#default`: the CLI package.
- `.#flash`: a wrapper app for built release-candidate flash targets.
- `.#rockpro64-release-bundle`: ROCKPro64 release-candidate bundle.
- `.#rockpro64-release-bundle-experimental`: ROCKPro64 experimental bundle.
- `.#raspberrypi3bplus-release-bundle`: Raspberry Pi 3 B+ release-candidate
  bundle.
- `nixosModules.rockpro64` and `nixosModules.rockpro64Bootable`.
- `nixosModules.raspberryPi3BPlus` and
  `nixosModules.raspberryPi3BPlusBootable`.

## NixOS Integration

Downstream flakes can import a bootable profile and add their own filesystems,
users, services, and deployment policy:

```nix
{
  inputs.bootswain.url = "git+https://github.com/caniko/bootswain.git";

  outputs = {
    nixpkgs,
    bootswain,
    ...
  }: {
    nixosConfigurations.rockpro64 = nixpkgs.lib.nixosSystem {
      system = "aarch64-linux";
      modules = [
        bootswain.nixosModules.rockpro64Bootable
        ./configuration.nix
      ];
    };

    nixosConfigurations.rpi3bplus = nixpkgs.lib.nixosSystem {
      system = "aarch64-linux";
      modules = [
        bootswain.nixosModules.raspberryPi3BPlusBootable
        ./configuration.nix
      ];
    };
  };
}
```

## Validation Evidence

Before promoting a board to stable, put real lab evidence in its board
directory: the validation run, serial logs, artifact manifest, and exact image
checksums.

For ROCKPro64, stable promotion requires:

- `validation/rockpro64/stable/validation-run.json`
- serial logs for every claimed scenario
- a successful build of `.#rockpro64-stable-release-bundle`

Raspberry Pi 3 B+ needs equivalent evidence and a successful build of
`.#raspberrypi3bplus-stable-release-bundle`. Pass `serial-u-boot-prompt`,
`family-dtb-check`, and `sd-extlinux-boot` in one run on an OS image, not the
firmware-only `sd-card.img`. Follow [the lab runbook](docs/src/raspberrypi3bplus/lab-runbook.md)
and store evidence under:

```text
validation/raspberrypi3bplus/stable/
```

## Development

Nix provides repeatable tooling. CI qualification runs in the `caniko/fleet`
superproject; only the Pages workflow lives here.

```sh
nix develop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- --deny warnings
cargo test --workspace
nix flake check --no-build
```

Serve the docs from `docs/`:

```sh
cd docs
mdbook serve
```

Serve the site from `website/`:

```sh
cd website
zola serve
```
