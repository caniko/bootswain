# ROCKPro64 Stable Evidence Drop

Place the hardware validation evidence for a stable ROCKPro64 release here
before building `.#rockpro64-stable-release-bundle`.

Required files:

- `validation-run.json`
- one serial log for every claimed scenario
- release artifact checksums from the exact candidate being promoted

The stable promotion derivation requires these scenarios to be present with
status `passed`:

- `spi-installer-menu`
- `spi-install`
- `spi-reinstall`
- `spi-erase`
- `spi-post-install-prompt`
- `no-bootable-media-ui`
- `recovery-console`
- `sd-uefi-boot`
- `sd-extlinux-boot`
- `emmc-uefi-boot`
- `emmc-extlinux-boot`
- `usb-uefi-boot`
- `usb-extlinux-boot`

The ordinary `rockpro64-release-bundle` remains a release candidate and does not
claim stable hardware support when these files are absent.
