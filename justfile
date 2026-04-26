# List the Nix-built flash targets through the flake app wrapper.
flash-targets:
    @mkdir -p "${XDG_CACHE_HOME:-{{justfile_directory()}}/.cache}"
    @XDG_CACHE_HOME="${XDG_CACHE_HOME:-{{justfile_directory()}}/.cache}" nix run .#flash -- --list-targets

# Flash a release artifact through the existing flake app wrapper.
[arg("allow_non_flashable", long="allow-non-flashable", value="--allow-non-flashable", help="Allow lab flashing of manifest artifacts not marked flashable")]
[arg("device", long="device", help="Block device path")]
[arg("json", long="json", value="--json", help="Emit JSON output")]
[arg("target", long="target", help="Flash target name")]
[arg("yes", long="yes", value="--yes", help="Skip confirmation prompt")]
[arg("verify_only", long="verify-only", value="--verify-only", help="Skip write and only verify")]
[arg("dry_run", long="dry-run", value="--dry-run", help="Validate without writing")]
[arg("verify", long="verify", value="--verify", help="Verify after flashing")]
flash target device dry_run="" verify="" verify_only="" json="" yes="" allow_non_flashable="":
    @mkdir -p "${XDG_CACHE_HOME:-{{justfile_directory()}}/.cache}"
    @XDG_CACHE_HOME="${XDG_CACHE_HOME:-{{justfile_directory()}}/.cache}" nix run .#flash -- --target "{{ target }}" --device "{{ device }}" {{ dry_run }} {{ verify }} {{ verify_only }} {{ json }} {{ yes }} {{ allow_non_flashable }}

# Run one ROCKPro64 serial validation scenario.
[arg("scenario", help="Scenario name from validation/rockpro64/lab-plan.json")]
[arg("port", long="port", help="Serial device path")]
[arg("baud", long="baud", help="Serial baud rate")]
[arg("out", long="out", help="Validation output directory")]
rockpro64-validate-scenario scenario port="/dev/ttyUSB0" baud="115200" out="bootswain-rockpro64-stable-run":
    nix develop --command cargo run -p bootswain-cli -- validate rockpro64-serial \
        --plan validation/rockpro64/lab-plan.json \
        --port "{{ port }}" \
        --baud "{{ baud }}" \
        --out "{{ out }}" \
        --allow-destructive-spi \
        --scenario "{{ scenario }}"

# Run the ROCKPro64 SPI install/reinstall/erase validation path.
# Use this only from legacy chainload or SD-bundled installer U-Boot, not installed SPI firmware.
[arg("port", long="port", help="Serial device path")]
[arg("baud", long="baud", help="Serial baud rate")]
[arg("out", long="out", help="Validation output directory")]
rockpro64-validate-spi port="/dev/ttyUSB0" baud="115200" out="bootswain-rockpro64-stable-run":
    nix develop --command cargo run -p bootswain-cli -- validate rockpro64-serial \
        --plan validation/rockpro64/lab-plan.json \
        --port "{{ port }}" \
        --baud "{{ baud }}" \
        --out "{{ out }}" \
        --allow-destructive-spi \
        --scenario spi-installer-menu \
        --scenario spi-install \
        --scenario spi-reinstall \
        --scenario spi-erase \
        --scenario spi-post-install-prompt \
        --scenario no-bootable-media-ui \
        --scenario recovery-console

# Run installed SPI firmware validation without destructive SPI actions.
[arg("port", long="port", help="Serial device path")]
[arg("baud", long="baud", help="Serial baud rate")]
[arg("out", long="out", help="Validation output directory")]
rockpro64-validate-installed-firmware port="/dev/ttyUSB0" baud="115200" out="bootswain-rockpro64-stable-run":
    nix develop --command cargo run -p bootswain-cli -- validate rockpro64-serial \
        --plan validation/rockpro64/lab-plan.json \
        --port "{{ port }}" \
        --baud "{{ baud }}" \
        --out "{{ out }}" \
        --scenario spi-post-install-prompt \
        --scenario no-bootable-media-ui \
        --scenario recovery-console

# Run the ROCKPro64 SD/eMMC/USB boot validation scenarios.
[arg("port", long="port", help="Serial device path")]
[arg("baud", long="baud", help="Serial baud rate")]
[arg("out", long="out", help="Validation output directory")]
rockpro64-validate-boot-media-scenario scenario port="/dev/ttyUSB0" baud="115200" out="bootswain-rockpro64-stable-run":
    nix develop --command cargo run -p bootswain-cli -- validate rockpro64-serial \
        --plan validation/rockpro64/lab-plan.json \
        --port "{{ port }}" \
        --baud "{{ baud }}" \
        --out "{{ out }}" \
        --scenario "{{ scenario }}"

# Run the ROCKPro64 SD/eMMC/USB boot validation scenarios.
# These scenarios hand off to boot media; prefer rockpro64-validate-boot-media-scenario with a board reset per scenario.
[arg("port", long="port", help="Serial device path")]
[arg("baud", long="baud", help="Serial baud rate")]
[arg("out", long="out", help="Validation output directory")]
rockpro64-validate-boot-media port="/dev/ttyUSB0" baud="115200" out="bootswain-rockpro64-stable-run":
    nix develop --command cargo run -p bootswain-cli -- validate rockpro64-serial \
        --plan validation/rockpro64/lab-plan.json \
        --port "{{ port }}" \
        --baud "{{ baud }}" \
        --out "{{ out }}" \
        --allow-destructive-spi \
        --scenario sd-uefi-boot \
        --scenario sd-extlinux-boot \
        --scenario emmc-uefi-boot \
        --scenario emmc-extlinux-boot \
        --scenario usb-uefi-boot \
        --scenario usb-extlinux-boot

# Run all ROCKPro64 stable-promotion hardware scenarios currently claimed.
[arg("port", long="port", help="Serial device path")]
[arg("baud", long="baud", help="Serial baud rate")]
[arg("out", long="out", help="Validation output directory")]
rockpro64-validate-stable port="/dev/ttyUSB0" baud="115200" out="bootswain-rockpro64-stable-run":
    nix develop --command cargo run -p bootswain-cli -- validate rockpro64-serial \
        --plan validation/rockpro64/lab-plan.json \
        --port "{{ port }}" \
        --baud "{{ baud }}" \
        --out "{{ out }}" \
        --allow-destructive-spi \
        --scenario spi-installer-menu \
        --scenario spi-install \
        --scenario spi-reinstall \
        --scenario spi-erase \
        --scenario spi-post-install-prompt \
        --scenario no-bootable-media-ui \
        --scenario recovery-console \
        --scenario sd-uefi-boot \
        --scenario sd-extlinux-boot \
        --scenario emmc-uefi-boot \
        --scenario emmc-extlinux-boot \
        --scenario usb-uefi-boot \
        --scenario usb-extlinux-boot
