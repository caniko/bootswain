# Bootswain

<!-- simit:badges:start -->

![CI](https://img.shields.io/badge/CI-managed-2088ff) [![Nix](https://img.shields.io/badge/Nix-managed-5277c3)](flake.nix) [![docs](https://img.shields.io/badge/docs-enabled-6f42c1)](docs)

<!-- simit:badges:end -->

`bootswain` is a Rust workspace for repeatable host-side board flashing,
firmware inspection, serial probing, and validation workflows around U-Boot.
The current focus is ROCKPro64 firmware work, with Raspberry Pi 3 B+ boot
partition tooling also present as release-candidate support.

The project is intentionally evidence-gated: release-candidate artifacts are
easy to build and inspect, but stable support claims require real validation
data under `validation/<board>/stable/`.

## What It Does

- Inspect raw and zstd-compressed image artifacts.
- Flash SD or removable block devices with dry-run and verification modes.
- Run reproducible ROCKPro64 USB and serial validation flows.
- Package ROCKPro64 RK3399 U-Boot, TF-A, SPI installer, and shared-storage
  images with Nix.
- Package a Raspberry Pi 3 B+ FAT boot partition image with Raspberry Pi
  firmware, DTB, and U-Boot.
- Expose NixOS modules for board boot-file installation and bootable profiles.

## Status

ROCKPro64 is the primary target. The SPI installer target is flashable as a
release candidate; shared-storage image flashing remains gated unless the
operator explicitly allows non-flashable artifacts for lab work.

Raspberry Pi 3 B+ support currently produces a release-candidate boot partition
image. It does not claim stable support until validation evidence is imported
under `validation/raspberrypi3bplus/stable/`.

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

Dry-run a Raspberry Pi 3 B+ boot partition flash:

```sh
nix run .#flash -- \
  --target raspberrypi3bplus-boot-partition \
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

Downstream flakes can import a bootable profile and layer their own filesystems,
users, services, and deployment policy on top:

```nix
{
  inputs.bootswain.url = "git+https://codeberg.org/caniko/bootswain.git";

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

Stable promotion is blocked until the relevant board directory contains real
lab evidence, including the validation run, serial logs, artifact manifest, and
exact image checksums.

For ROCKPro64, stable promotion requires:

- `validation/rockpro64/stable/validation-run.json`
- serial logs for every claimed scenario
- a successful build of `.#rockpro64-stable-release-bundle`

For Raspberry Pi 3 B+, stable promotion requires equivalent evidence under:

```text
validation/raspberrypi3bplus/stable/
```

## Development

The repository uses Nix for repeatable tooling and generated Forgejo CI for
testing. The generated CI workflows run package tests and clippy through
`nix develop`, and run `nix flake check`.

Useful local commands:

```sh
nix develop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- --deny warnings
cargo test --workspace
nix flake check --no-build
```

Documentation lives in `docs/` and can be served with:

```sh
cd docs
mdbook serve
```

The website lives in `website/` and can be served with:

```sh
cd website
zola serve
```
