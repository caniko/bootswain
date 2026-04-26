# ROCKPro64 Probing

The first ROCKPro64 probe flow matches the USB investigation protocol used
during Tow-Boot replacement work:

1. wait for the U-Boot prompt
2. run `usb start`
3. run `usb tree`
4. run `usb reset`
5. run `usb tree`
6. write raw logs plus machine-readable JSON results

Run one trial:

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

Each probe run writes:

- `summary.json` in the requested output directory
- `trial.json` plus `serial.log` under `trial-N/` for each trial

`trial.json` includes stage-level results for autoboot, `usb start`,
`usb tree`, `usb reset`, and failure stage classification. The JSON files and
`--json` output are the current public machine-readable interface.

## UART Caveat

ROCKPro64 serial wiring can interfere with power-on if board RX is connected too
early.

Use this as the default operator baseline while probing:

- connect only `GND` and board `TX` during power-on
- leave board `RX` / pin 10 disconnected until U-Boot is already up
- use `115200` unless deliberately testing a different serial policy
