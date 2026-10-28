# Raspberry Pi 3 B+ Lab Runbook

Hardware: a Raspberry Pi 3 B+, a 3.3 V USB-UART adapter wired as described in the
[operator notes](./operator-notes.md), and an SD card.

1. Build and flash the release-candidate image:

   ```sh
   nix build .#raspberrypi3bplus-release-bundle
   nix run .#flash -- --target raspberrypi3bplus-sd-card --device /dev/sdX --verify
   ```

2. Open the serial adapter at 115200 8N1 and power the board.
3. Run each scenario and keep its serial log:

   ```sh
   just rpi3bplus-validate-scenario serial-u-boot-prompt
   just rpi3bplus-validate-scenario family-dtb-check
   just rpi3bplus-validate-scenario sd-extlinux-boot
   ```

   Equivalent without `just`:

   ```sh
   bootswain validate serial --plan validation/raspberrypi3bplus/lab-plan.json
   ```

4. Copy `validation-run.json` and the serial logs into
   `validation/raspberrypi3bplus/stable/`.
5. Promote:

   ```sh
   nix build .#raspberrypi3bplus-stable-release-bundle
   ```

   The build fails with `missing stable hardware validation evidence` until the
   evidence exists, and with `missing passing stable scenario: <name>` if any of
   `serial-u-boot-prompt`, `family-dtb-check`, `sd-extlinux-boot` did not pass.

The SD image layer (NixOS) is provided by ignitix. CI qualification of this
repository happens in `caniko/fleet`.
