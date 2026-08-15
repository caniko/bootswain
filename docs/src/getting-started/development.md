# Development

This repository ships a Nix flake and a Cargo workspace.

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

## Documentation Sites

Build the presentation site:

```sh
nix build .#website
```

Build the mdBook documentation:

```sh
nix build .#docs
```

Build the combined GitHub Pages output:

```sh
nix build .#site
```
