+++
title = "Bootswain"

[extra]
tagline = "ROCKPro64 flash, probe, validation, and firmware packaging workflows."
subtitle = "Bootswain turns host-side flashing, serial evidence capture, and RK3399 firmware release packaging into reproducible Rust and Nix workflows."
install = "nix develop"

[[extra.features]]
title = "Flash With Guardrails"
body = "Inspect installer artifacts, dry-run writes, reject mounted targets, and flash release-candidate media through explicit commands."

[[extra.features]]
title = "Probe Reproducibly"
body = "Capture ROCKPro64 serial logs and machine-readable trial summaries for USB, bootflow, and validation work."

[[extra.features]]
title = "Package Firmware"
body = "Build RK3399 U-Boot, TF-A, SPI installer, shared-storage images, checksums, manifests, and provenance in Nix."

[[extra.features]]
title = "Gate Stable Claims"
body = "Keep ROCKPro64 support claims tied to imported hardware evidence and release artifacts that name what was actually tested."

[[extra.features]]
title = "Compose NixOS"
body = "Expose ROCKPro64 NixOS modules for boot artifact integration and downstream board-specific systems."

[[extra.features]]
title = "Publish Together"
body = "Build the presentation site and mdBook documentation as one Codeberg Pages output."
+++
