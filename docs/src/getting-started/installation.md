# Installation

Bootswain is developed and distributed as a Nix flake and Rust workspace.

Enter the development environment from a checkout:

```sh
nix develop
```

Build the Rust workspace:

```sh
cargo build --workspace
```

Run the test suite:

```sh
cargo test --workspace
```

The flake exposes release and utility packages, including the flash app and
ROCKPro64 firmware bundles.

List available flash targets:

```sh
nix run .#flash -- --list-targets
```
