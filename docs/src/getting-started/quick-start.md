# Quick Start

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

Dry-run the release-candidate ROCKPro64 SPI installer target:

```sh
nix run .#flash -- \
  --target rockpro64-spi-installer \
  --device /dev/sdb \
  --dry-run
```

Run one ROCKPro64 USB probe trial:

```sh
bootswain probe rockpro64-usb \
  --image /tmp/result-rockpro64-stock-2026.04/spi.installer.img \
  --port /dev/ttyUSB0 \
  --out ./probe-output
```
