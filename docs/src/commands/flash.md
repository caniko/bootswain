# Image Inspection and Flashing

Bootswain can inspect `.img` and `.img.zst` artifacts, then flash image files to
whole-disk targets from Linux.

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

Validate a flash target without writing:

```sh
bootswain flash sd \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --device /dev/sdb \
  --dry-run \
  --json
```

Use Nix-built release targets through the flash app:

```sh
nix run .#flash -- --list-targets
nix run .#flash -- \
  --target rockpro64-spi-installer \
  --device /dev/sdb \
  --dry-run
```

The release-candidate SPI installer is flashable. Shared-storage images remain
gated until matching SD/eMMC hardware validation evidence is imported.
