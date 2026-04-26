# Bootswain

`bootswain` is a Rust workspace for repeatable ROCKPro64-first flash, probe,
and firmware packaging workflows around stock U-Boot. ROCKPro64 firmware
artifacts are release candidates until real hardware evidence is imported under
`validation/rockpro64/stable/`; stable support claims are not emitted by the
default release bundle.

It replaces the manual host steps we have been using for:

- inspecting installer and firmware artifacts
- flashing SD media safely
- capturing serial logs
- running the ROCKPro64 USB probe sequence reproducibly
- packaging real RK3399 U-Boot, TF-A, SPI installer, and shared-storage images
  in Nix

## Current scope

V1 is intentionally narrow:

- Linux-only
- path-based inputs for images and serial ports
- built-in ROCKPro64 support only
- release-candidate flashing is enabled for the SPI installer target
- shared-storage image flashing remains gated until SD/eMMC validation evidence
  is imported

The first probe flow matches the investigation protocol already used in
Tow-Boot:

1. wait for the U-Boot prompt
2. run `usb start`
3. run `usb tree`
4. run `usb reset`
5. run `usb tree`
6. write raw logs plus machine-readable JSON results

## ROCKPro64 UART caveat

ROCKPro64 serial wiring can interfere with power-on if board RX is connected too
early.

Use this as the default operator baseline while probing:

- connect only `GND` + board `TX` during power-on
- leave board `RX` / pin 10 disconnected until U-Boot is already up
- use `115200` unless you deliberately test a different serial policy

## Commands

Inspect an image:

```sh
bootswain image inspect --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img
```

Inspect an image with JSON output:

```sh
bootswain image inspect \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --json
```

Flash an SD card:

```sh
bootswain flash sd \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --device /dev/sdb
```

Use the convenience wrapper from the dev shell:

```sh
nix develop
just flash-targets
just flash \
  --target rockpro64-spi-installer \
  --device /dev/sdX \
  --verify
```

List the Nix-built flash targets:

```sh
nix run .#flash -- --list-targets
```

Dry-run the release-candidate ROCKPro64 SPI installer target:

```sh
nix run .#flash -- \
  --target rockpro64-spi-installer \
  --device /dev/sdb \
  --dry-run
```

Validate a flash target without writing:

```sh
bootswain flash sd \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --device /dev/sdb \
  --dry-run \
  --json
```

Run one ROCKPro64 USB probe trial:

```sh
bootswain probe rockpro64-usb \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --port /dev/ttyUSB0 \
  --out ./probe-output
```

Run three cold-boot trials:

```sh
bootswain probe rockpro64-usb \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --port /dev/ttyUSB0 \
  --repeat 3 \
  --out ./probe-output
```

Adjust probe timeouts and emit JSON to stdout:

```sh
bootswain probe rockpro64-usb \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --port /dev/ttyUSB0 \
  --prompt-timeout-secs 30 \
  --command-timeout-secs 15 \
  --json
```

## Output

Each probe run writes:

- `summary.json` in the requested output directory
- `trial.json` plus `serial.log` under `trial-N/` for each trial

`trial.json` includes stage-level results for:

- autoboot
- `usb start`
- `usb tree`
- `usb reset`
- failure stage classification

The JSON files and `--json` output are the current public machine-readable
interface.

## Development

## Downstream NixOS ROCKPro64 config

Downstream flakes can import the bootable ROCKPro64 profile and layer their own
filesystems, users, services, and deployment policy on top:

```nix
{
  inputs.bootswain.url = "github:caniko/bootswain";

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
  };
}
```

The profile enables the bootswain ROCKPro64 integration, disables GRUB, enables
NixOS generic extlinux output, sets `nixpkgs.hostPlatform` to `aarch64-linux`,
and adds a serial console matching the bootswain UART policy. The lower-level
`bootswain.nixosModules.rockpro64` module remains available when a downstream
flake wants to opt into those settings manually.

This repository ships a Nix flake and a Cargo workspace:

```sh
nix develop
cargo build --workspace
cargo test --workspace
```

CI runs:

- `cargo build`
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo fmt --check`
- `nix flake check`

Stable ROCKPro64 promotion additionally requires:

- `validation/rockpro64/stable/validation-run.json`
- full serial logs for every claimed scenario
- a successful build of `.#rockpro64-stable-release-bundle`
