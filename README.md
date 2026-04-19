# Bootswain

`bootswain` is a host-side Rust workspace for repeatable ROCKPro64-first flash
and probe workflows around stock U-Boot. It does not replace U-Boot on-device.
It replaces the manual host steps we have been using for:

- inspecting installer and firmware artifacts
- flashing SD media safely
- capturing serial logs
- running the ROCKPro64 USB probe sequence reproducibly

## Current scope

V1 is intentionally narrow:

- Linux-only
- host-side only
- path-based inputs for images and serial ports
- built-in ROCKPro64 support only

The first probe flow matches the investigation protocol already used in
Tow-Boot:

1. wait for the U-Boot prompt
2. run `usb tree`
3. run `usb reset`
4. run `usb tree`
5. write raw logs plus machine-readable JSON results

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

Flash an SD card:

```sh
bootswain flash sd \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --device /dev/sdb
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

## Output

Each probe run writes:

- `summary.json` in the requested output directory
- `trial.json` plus `serial.log` under `trial-N/` for each trial

The JSON output is the current public machine-readable interface for V1.

## Development

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

