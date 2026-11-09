# Bootswain vision

Repeatable host-side firmware work for single-board computers: inspect
installer and firmware artifacts, flash SD or removable media safely, capture
serial logs, run scripted U-Boot validation flows, and package the images in
Nix. ROCKPro64 is the primary target; Raspberry Pi 3 B+ is hard-pushed next to
it. Operators who reinstall a board's SPI or SD card and then have to prove it
booted are the users.

## Problem

Board bring-up is normally a sequence of remembered commands: `dd` an image at
the right offset, hope the card is the right one, unplug and replug, attach a
serial cable, watch a prompt scroll away, and write down what happened. Nothing
reproduces the next attempt, and nothing distinguishes "the build succeeded"
from "the board booted". Bootswain makes flashing and probing bounded and
verified, and makes a support claim an evidence artifact.

## Principles

- Evidence gates escalation. Artifacts are release candidates until real
  hardware logs are imported under `validation/<board>/stable/`; the default
  bundle emits `tested: false`.
- Verification is byte-level. Flash can dry-run, and verify bypasses the page
  cache, buffers writes and checks the decoded payload size - including all
  zstd frames, not just the first.
- Writes are confirmed before they are trusted. Unmounted-boot guards, device
  identity checks and preserved evidence are part of the flash and probe paths.
- Claims are scoped to what has logs. A stable release does not claim NVMe OS
  boot, network boot, USB gadget mode, persistent environment edits or other
  boards unless matching logs are added first.
- Rejections are explicit. The stable promotion derivation fails when
  `validation-run.json` is missing, a claimed scenario is not `passed`, or a
  referenced serial log does not exist.
- Mechanism is separated from policy: `bootswain-core` (artifacts and
  validation), `bootswain-flash`, `bootswain-probe` and the CLI, with Nix
  packaging firmware and exposing NixOS modules.

## Scope

- Inspect raw and zstd-compressed image artifacts.
- Flash SD or removable block devices with dry-run and verification modes.
- Package RK3399 U-Boot, TF-A, SPI installer and shared-storage images for
  ROCKPro64; package an MBR SD card image with Raspberry Pi firmware, Pi 3
  family DTBs and U-Boot.
- Native Linux platform probe, serial console capture and reusable U-Boot prompt
  detectors.
- Release-candidate Pi 5 U-Boot UEFI and ROCK 5A EDK2 artifacts with a
  board-bound read-only probe.
- NixOS modules for board boot-file installation and bootable profiles.

## Non-goals

- macOS or Windows host tooling; V1 is Linux-only.
- Being a general installer for arbitrary boards without a matching lab plan and
  validation evidence.
- Hiding a failed or incomplete run. Unknown and unclaimed states stay visible
  in the release output.
- Managing the running OS on the board; bootswain installs boot files and
  validates that they boot.

## Direction

Pi 3 B+ moved from best-effort to hardware-validated by importing stable
evidence and calibrating the lab plan against a real unit, and its first serial
lessons are documented for the next board. The same shape - lab plan, recipes,
runbook, stable gate - is the template for further boards; UEFI appliance work
stays at release candidate until its physical qualification gates pass.

## Place in the fleet

Canix pins it as a flake input and `root/profiles/server/default.nix` notes that
EFI comes from bootswain (`osBootProtocol = "efi"`). It is one of the projects
the fleet's Rust/Nix toolchain (Simit-generated CI, `harbor-rs`, treefmt) builds
and releases.
