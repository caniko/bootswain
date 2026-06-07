{
  pkgs,
  bootswain,
  raspberryPi3BPlusReleaseBundle,
  rockpro64ReleaseBundle,
  rockpro64ReleaseBundleExperimental,
}:
let
  # Flake evaluation ignores untracked files, so keep this in sync with repo-root justfile.
  justfileText = ''
    # List the Nix-built flash targets through the flake app wrapper.
    flash-targets:
        @mkdir -p "''${XDG_CACHE_HOME:-{{ justfile_directory() }}/.cache}"
        @XDG_CACHE_HOME="''${XDG_CACHE_HOME:-{{ justfile_directory() }}/.cache}" nix run .#flash -- --list-targets

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
        @mkdir -p "''${XDG_CACHE_HOME:-{{ justfile_directory() }}/.cache}"
        @XDG_CACHE_HOME="''${XDG_CACHE_HOME:-{{ justfile_directory() }}/.cache}" nix run .#flash -- --target "{{ target }}" --device "{{ device }}" {{ dry_run }} {{ verify }} {{ verify_only }} {{ json }} {{ yes }} {{ allow_non_flashable }}
  '';

  justfileCheck = pkgs.runCommand "justfile-check" {
    nativeBuildInputs = [ pkgs.just ];
  } ''
    mkdir -p "$out"
    cp ${pkgs.writeText "bootswain-justfile" justfileText} "$TMPDIR/justfile"

    just --justfile "$TMPDIR/justfile" --unstable --fmt --check
    just --justfile "$TMPDIR/justfile" --list > "$out/list.txt"
    just --justfile "$TMPDIR/justfile" --usage flash > "$out/usage.txt"

    grep -q 'flash-targets' "$out/list.txt"
    grep -q 'flash ' "$out/list.txt"
    grep -q -- '--target' "$out/usage.txt"
    grep -q -- '--device' "$out/usage.txt"
    grep -q -- '--dry-run' "$out/usage.txt"
    grep -q -- '--verify' "$out/usage.txt"
    grep -q -- '--verify-only' "$out/usage.txt"
    grep -q -- '--json' "$out/usage.txt"
    grep -q -- '--yes' "$out/usage.txt"
    grep -q -- '--allow-non-flashable' "$out/usage.txt"
  '';

  flashApp = pkgs.writeShellApplication {
    name = "bootswain-flash";
    text = ''
      set -euo pipefail

      show_targets() {
        cat <<'EOF'
      available targets:
        raspberrypi3bplus-boot-partition    channel: release-candidate artifact: boot-partition-image
        rockpro64-spi-installer               channel: release-candidate artifact: spi-installer
        rockpro64-spi-installer-experimental  channel: experimental artifact: spi-installer
        rockpro64-shared-disk-image           channel: release-candidate artifact: shared-disk-image
      EOF
      }

      usage() {
        cat <<'EOF'
      Usage:
        bootswain-flash --target <target> --device <device> [--dry-run] [--verify] [--verify-only] [--json] [--yes] [--allow-non-flashable]
        bootswain-flash --list-targets

      Examples:
        nix run .#flash -- --target rockpro64-spi-installer --device /dev/sdX --dry-run
        nix run .#flash -- --target rockpro64-spi-installer-experimental --device /dev/sdX --dry-run
        nix run .#flash -- --target rockpro64-spi-installer --device /dev/sdX --verify --yes
      EOF
      }

      target=""
      channel=""
      device=""
      passthrough=()

      while [[ $# -gt 0 ]]; do
        case "$1" in
          --list-targets)
            show_targets
            exit 0
            ;;
          --target)
            if [[ $# -lt 2 ]]; then
              echo "error: --target requires a value" >&2
              exit 2
            fi
            target="$2"
            shift 2
            ;;
          --device)
            if [[ $# -lt 2 ]]; then
              echo "error: --device requires a value" >&2
              exit 2
            fi
            device="$2"
            shift 2
            ;;
          --dry-run|--verify|--verify-only|--json|--yes|--allow-non-flashable)
            passthrough+=("$1")
            shift
            ;;
          -h|--help)
            usage
            echo
            show_targets
            exit 0
            ;;
          *)
            echo "error: unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
        esac
      done

      if [[ -z "$target" ]]; then
        echo "error: --target is required" >&2
        usage >&2
        exit 2
      fi
      if [[ -z "$device" ]]; then
        echo "error: --device is required" >&2
        usage >&2
        exit 2
      fi

      requested_target="$target"
      case "$target" in
        rockpro64|rockpro64-rc)
          target="rockpro64-spi-installer"
          ;;
        rockpro64-experimental)
          target="rockpro64-spi-installer-experimental"
          ;;
        raspberrypi3bplus|rpi3bplus)
          target="raspberrypi3bplus-boot-partition"
          ;;
      esac

      case "$target" in
        raspberrypi3bplus-boot-partition)
          bundle="${raspberryPi3BPlusReleaseBundle}"
          artifact="boot-partition-image"
          channel="release-candidate"
          ;;
        rockpro64-spi-installer)
          bundle="${rockpro64ReleaseBundle}"
          artifact="spi-installer"
          channel="release-candidate"
          ;;
        rockpro64-spi-installer-experimental)
          bundle="${rockpro64ReleaseBundleExperimental}"
          artifact="spi-installer"
          channel="experimental"
          ;;
        rockpro64-shared-disk-image)
          bundle="${rockpro64ReleaseBundle}"
          artifact="shared-disk-image"
          channel="release-candidate"
          ;;
        *)
          echo "error: unknown target: $target" >&2
          show_targets >&2
          exit 2
          ;;
      esac

      if [[ "$requested_target" != "$target" ]]; then
        echo "bootswain flash alias: $requested_target -> $target" >&2
      fi
      echo "bootswain flash target: $target" >&2
      echo "bootswain flash channel: $channel" >&2
      echo "bootswain flash artifact: $artifact" >&2
      echo "bootswain manifest bundle: $bundle" >&2
      echo "bootswain target device: $device" >&2
      if [[ ''${#passthrough[@]} -gt 0 ]]; then
        printf 'bootswain flash flags:' >&2
        printf ' %s' "''${passthrough[@]}" >&2
        printf '\n' >&2
      else
        echo "bootswain flash flags: none" >&2
      fi

      exec ${bootswain}/bin/bootswain flash sd \
        --manifest "$bundle/release.json" \
        --artifact "$artifact" \
        --device "$device" \
        "''${passthrough[@]}"
    '';
  };

  flashAppWrapperCheck = pkgs.runCommand "flash-app-wrapper-check" { } ''
    mkdir -p "$out"

    ${flashApp}/bin/bootswain-flash --list-targets > "$out/targets.txt"
    grep -q 'raspberrypi3bplus-boot-partition' "$out/targets.txt"
    grep -q 'rockpro64-spi-installer' "$out/targets.txt"
    grep -q 'rockpro64-spi-installer-experimental' "$out/targets.txt"
    grep -q 'rockpro64-shared-disk-image' "$out/targets.txt"

    set +e
    ${flashApp}/bin/bootswain-flash --device /dev/null \
      > "$out/missing-target.out" 2> "$out/missing-target.err"
    status=$?
    set -e
    test "$status" -eq 2
    grep -q -- '--target is required' "$out/missing-target.err"

    set +e
    ${flashApp}/bin/bootswain-flash --target rockpro64-spi-installer \
      > "$out/missing-device.out" 2> "$out/missing-device.err"
    status=$?
    set -e
    test "$status" -eq 2
    grep -q -- '--device is required' "$out/missing-device.err"

    set +e
    ${flashApp}/bin/bootswain-flash --target nope --device /dev/null \
      > "$out/unknown-target.out" 2> "$out/unknown-target.err"
    status=$?
    set -e
    test "$status" -eq 2
    grep -q 'unknown target: nope' "$out/unknown-target.err"
    grep -q 'rockpro64-spi-installer' "$out/unknown-target.err"
    grep -q 'raspberrypi3bplus-boot-partition' "$out/unknown-target.err"

    set +e
    ${flashApp}/bin/bootswain-flash \
      --target rpi3bplus \
      --device /dev/null \
      --dry-run \
      > "$out/rpi3bplus-alias.out" 2> "$out/rpi3bplus-alias.err"
    status=$?
    set -e
    test "$status" -ne 0
    grep -q 'bootswain flash alias: rpi3bplus -> raspberrypi3bplus-boot-partition' "$out/rpi3bplus-alias.err"
    grep -q 'bootswain flash target: raspberrypi3bplus-boot-partition' "$out/rpi3bplus-alias.err"
    grep -q 'bootswain flash artifact: boot-partition-image' "$out/rpi3bplus-alias.err"
    grep -q 'target is not a block device: /dev/null' "$out/rpi3bplus-alias.err"
    if grep -q 'not marked flashable' "$out/rpi3bplus-alias.err"; then
      echo "raspberry pi 3 b+ target unexpectedly failed manifest gating" >&2
      exit 1
    fi

    set +e
    ${flashApp}/bin/bootswain-flash \
      --target rockpro64-experimental \
      --device /dev/null \
      --dry-run \
      > "$out/alias-target.out" 2> "$out/alias-target.err"
    status=$?
    set -e
    test "$status" -ne 0
    grep -q 'bootswain flash alias: rockpro64-experimental -> rockpro64-spi-installer-experimental' "$out/alias-target.err"
    grep -q 'bootswain flash target: rockpro64-spi-installer-experimental' "$out/alias-target.err"
    grep -q 'bootswain flash channel: experimental' "$out/alias-target.err"
    grep -q 'target is not a block device: /dev/null' "$out/alias-target.err"

    set +e
    ${flashApp}/bin/bootswain-flash \
      --target rockpro64-spi-installer \
      --device /dev/null \
      --dry-run \
      > "$out/stable-dry-run.out" 2> "$out/stable-dry-run.err"
    status=$?
    set -e
    test "$status" -ne 0
    grep -q 'bootswain flash target: rockpro64-spi-installer' "$out/stable-dry-run.err"
    grep -q 'bootswain flash channel: release-candidate' "$out/stable-dry-run.err"
    grep -q 'bootswain flash artifact: spi-installer' "$out/stable-dry-run.err"
    grep -q 'bootswain flash flags: --dry-run' "$out/stable-dry-run.err"
    grep -q 'target is not a block device: /dev/null' "$out/stable-dry-run.err"
    if grep -q 'not marked flashable' "$out/stable-dry-run.err"; then
      echo "release-candidate target unexpectedly failed manifest gating" >&2
      exit 1
    fi

    set +e
    ${flashApp}/bin/bootswain-flash \
      --target rockpro64-spi-installer \
      --device /dev/null \
      --verify \
      --json \
      --yes \
      > "$out/passthrough.out" 2> "$out/passthrough.err"
    status=$?
    set -e
    test "$status" -ne 0
    grep -q 'bootswain flash flags: --verify --json --yes' "$out/passthrough.err"
    grep -q 'target is not a block device: /dev/null' "$out/passthrough.err"
    if grep -q 'not marked flashable' "$out/passthrough.err"; then
      echo "release-candidate passthrough unexpectedly failed manifest gating" >&2
      exit 1
    fi

    set +e
    ${flashApp}/bin/bootswain-flash \
      --target rockpro64-spi-installer-experimental \
      --device /dev/null \
      --dry-run \
      > "$out/experimental-dry-run.out" 2> "$out/experimental-dry-run.err"
    status=$?
    set -e
    test "$status" -ne 0
    grep -q 'bootswain flash target: rockpro64-spi-installer-experimental' "$out/experimental-dry-run.err"
    grep -q 'bootswain flash channel: experimental' "$out/experimental-dry-run.err"
    grep -q 'bootswain flash artifact: spi-installer' "$out/experimental-dry-run.err"
    grep -q 'bootswain flash flags: --dry-run' "$out/experimental-dry-run.err"
    grep -q 'target is not a block device: /dev/null' "$out/experimental-dry-run.err"
    if grep -q 'not marked flashable' "$out/experimental-dry-run.err"; then
      echo "experimental target unexpectedly failed manifest gating" >&2
      exit 1
    fi

    set +e
    ${flashApp}/bin/bootswain-flash \
      --target rockpro64-shared-disk-image \
      --device /dev/null \
      --dry-run \
      --allow-non-flashable \
      > "$out/shared-disk-override.out" 2> "$out/shared-disk-override.err"
    status=$?
    set -e
    test "$status" -ne 0
    grep -q 'bootswain flash target: rockpro64-shared-disk-image' "$out/shared-disk-override.err"
    grep -q 'bootswain flash artifact: shared-disk-image' "$out/shared-disk-override.err"
    grep -q 'bootswain flash flags: --dry-run --allow-non-flashable' "$out/shared-disk-override.err"
    grep -q 'target is not a block device: /dev/null' "$out/shared-disk-override.err"
    if grep -q 'not marked flashable' "$out/shared-disk-override.err"; then
      echo "shared disk override unexpectedly failed manifest gating" >&2
      exit 1
    fi
  '';
in
{
  apps = {
    flash = {
      type = "app";
      program = "${flashApp}/bin/bootswain-flash";
      meta.description = "Build and flash a selected bootswain release artifact through bootswain flash sd";
    };
  };

  checks = {
    flash-app-wrapper = flashAppWrapperCheck;
    justfile = justfileCheck;
  };
}
