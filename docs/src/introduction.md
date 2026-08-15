# Bootswain

`bootswain` is a Rust workspace for repeatable ROCKPro64-first flash, probe,
validation, and firmware packaging workflows around U-Boot.

The project replaces manual host steps for:

- inspecting installer and firmware artifacts
- flashing SD media safely
- capturing serial logs
- running reproducible ROCKPro64 probe and validation flows
- packaging RK3399 U-Boot, TF-A, SPI installer, and shared-storage images in Nix

ROCKPro64 firmware artifacts are release candidates until real hardware evidence
is imported under `validation/rockpro64/stable/`. The default release bundle does
not emit stable support claims.

## Current Scope

V1 is intentionally narrow:

- Linux-only host tooling
- path-based inputs for images and serial ports
- built-in ROCKPro64 support
- release-candidate flashing for the SPI installer target
- evidence-gated shared-storage and stable hardware claims

The source repository is hosted at
[github.com/caniko/bootswain](https://github.com/caniko/bootswain).
