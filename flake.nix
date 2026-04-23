{
  description = "bootswain — ROCKPro64-first host-side flash and probe tooling plus firmware scaffolding";

  inputs = {
    rs-harbor.url = "git+ssh://git@codeberg.org/caniko/rs-harbor.git";
    nixpkgs.follows = "rs-harbor/nixpkgs";
    rust-overlay.follows = "rs-harbor/rust-overlay";
    crane.follows = "rs-harbor/crane";
    flake-utils.follows = "rs-harbor/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rs-harbor,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        lib = pkgs.lib;
        toolchain = rs-harbor.lib.mkToolchain { inherit pkgs; };
        cross = rs-harbor.lib.mkCross { inherit pkgs system; };
        craneLib = toolchain.craneLib;
        workspaceRoot = toString ./.;
        src = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            let
              relativePath = lib.removePrefix "${workspaceRoot}/" (toString path);
            in
            (craneLib.filterCargoSources path type)
            || lib.hasPrefix "docs/" relativePath
            || lib.hasPrefix "firmware/" relativePath
            || lib.hasPrefix "validation/" relativePath;
        };
        rockpro64FirmwareDir = builtins.path {
          path = ./firmware/rockpro64;
          name = "rockpro64-firmware";
        };
        validationDir = builtins.path {
          path = ./validation;
          name = "bootswain-validation-fixtures";
        };
        qemuArm64Uboot = pkgs.pkgsCross.aarch64-multiplatform.ubootQemuAarch64;

        rockpro64Uboot = pkgs.linkFarm "rockpro64-uboot" [
          {
            name = "README.md";
            path = "${rockpro64FirmwareDir}/README.md";
          }
          {
            name = "contract.md";
            path = "${rockpro64FirmwareDir}/contract.md";
          }
          {
            name = "boot.cmd";
            path = "${rockpro64FirmwareDir}/boot.cmd";
          }
          {
            name = "uboot.env";
            path = "${rockpro64FirmwareDir}/uboot.env";
          }
          {
            name = "u-boot.config.fragment";
            path = "${rockpro64FirmwareDir}/u-boot.config.fragment";
          }
        ];

        rockpro64SpiFirmware = pkgs.linkFarm "rockpro64-spi-firmware" [
          {
            name = "README.md";
            path = "${rockpro64FirmwareDir}/README.md";
          }
          {
            name = "contract.md";
            path = "${rockpro64FirmwareDir}/contract.md";
          }
          {
            name = "spi-layout.md";
            path = "${rockpro64FirmwareDir}/spi-layout.md";
          }
          {
            name = "boot.cmd";
            path = "${rockpro64FirmwareDir}/boot.cmd";
          }
          {
            name = "uboot.env";
            path = "${rockpro64FirmwareDir}/uboot.env";
          }
          {
            name = "u-boot.config.fragment";
            path = "${rockpro64FirmwareDir}/u-boot.config.fragment";
          }
        ];

        rockpro64SpiInstallerImg = pkgs.writeText "rockpro64-spi-installer.img" ''
          bootswain ROCKPro64 SPI installer image scaffold

          Status: scaffold only
          Hardware validation: not performed
          Board: pine64-rockpro64
          SoC: rockchip-rk3399

          This file is a release placeholder for the SD-bootable installer image.
          It records the intended contract and must not be mistaken for a
          validated bootable artifact.

          Source files:
          - firmware/rockpro64/contract.md
          - firmware/rockpro64/spi-layout.md
          - firmware/rockpro64/boot.cmd
          - firmware/rockpro64/uboot.env
          - firmware/rockpro64/u-boot.config.fragment
        '';

        rockpro64SharedDiskImageImg = pkgs.writeText "rockpro64-shared-disk-image.img" ''
          bootswain ROCKPro64 shared-storage image scaffold

          Status: scaffold only
          Hardware validation: not performed
          Board: pine64-rockpro64
          SoC: rockchip-rk3399

          This file is a release placeholder for the shared-storage disk image.
          It documents the intended firmware contract without claiming a
          validated bootable image.

          Source files:
          - firmware/rockpro64/contract.md
          - firmware/rockpro64/spi-layout.md
          - firmware/rockpro64/boot.cmd
          - firmware/rockpro64/uboot.env
          - firmware/rockpro64/u-boot.config.fragment
        '';

        rockpro64ReleaseManifest = pkgs.writeText "rockpro64-release-manifest.json" (builtins.toJSON {
          schema_version = 1;
          release = "2026.04-rockpro64-scaffold.0";
          board = "rock-pro64";
          sources = {
            u_boot = "v2026.04";
            trusted_firmware_a = "pinned-by-next-firmware-build-phase";
            bootswain = "workspace";
          };
          storage_layout = {
            spi_size_bytes = 16777216;
            spi_firmware_offset_bytes = 0;
            spi_firmware_size_bytes = null;
            environment_offset_bytes = null;
            environment_size_bytes = null;
            shared_storage_firmware_partition = "bootswain-firmware";
          };
          boot_policy = {
            default_order = [
              "sd"
              "emmc"
              "usb"
              "nvme"
            ];
            preferred_protocols = [
              "uefi"
              "extlinux"
            ];
            no_bootable_media_behavior = "show boot menu, diagnostics, and U-Boot shell over 115200 serial";
          };
          environment_policy = {
            persistent = false;
            location = null;
            stale_environment_safe = true;
            notes = "Scaffold defaults to noenv behavior until hardware-validated persistent env offsets exist.";
          };
          artifacts = [
            {
              kind = "spi-installer";
              path = "rockpro64-spi-installer.img";
              compression = "none";
              size_bytes = 0;
              sha256 = "scaffold-not-computed";
              required_device_size_bytes = null;
              flashable = false;
            }
            {
              kind = "shared-disk-image";
              path = "rockpro64-shared-disk-image.img";
              compression = "none";
              size_bytes = 0;
              sha256 = "scaffold-not-computed";
              required_device_size_bytes = null;
              flashable = false;
            }
            {
              kind = "release-manifest";
              path = "rockpro64-release-manifest.json";
              compression = "none";
              size_bytes = 0;
              sha256 = "scaffold-not-computed";
              required_device_size_bytes = null;
              flashable = false;
            }
            {
              kind = "provenance";
              path = "provenance.json";
              compression = "none";
              size_bytes = 0;
              sha256 = "scaffold-not-computed";
              required_device_size_bytes = null;
              flashable = false;
            }
          ];
          validation_claims = [
            {
              target = "sd";
              protocols = [
                "u-boot-shell"
              ];
              tested = false;
              notes = "Scaffold only; no hardware validation has been performed.";
            }
          ];
          unsupported_paths = [
            "phone/tablet volume-button shortcuts"
            "USB mass-storage gadget mode"
            "advertising NVMe/eMMC/USB boot support before lab validation"
          ];
        });

        rockpro64ChecksumsProvenance = pkgs.runCommand "rockpro64-checksums-provenance" {
          nativeBuildInputs = [
            pkgs.coreutils
          ];
        } ''
          mkdir -p "$out"

          {
            echo "# SHA-256 for firmware/rockpro64 source files"
            sha256sum \
              ${rockpro64FirmwareDir}/README.md \
              ${rockpro64FirmwareDir}/contract.md \
              ${rockpro64FirmwareDir}/spi-layout.md \
              ${rockpro64FirmwareDir}/boot.cmd \
              ${rockpro64FirmwareDir}/uboot.env \
              ${rockpro64FirmwareDir}/u-boot.config.fragment
          } > "$out/checksums.txt"

          cat > "$out/provenance.json" <<EOF
${builtins.toJSON {
  board = "pine64-rockpro64";
  soc = "rockchip-rk3399";
  release_stage = "scaffold";
  hardware_validation = {
    performed = false;
    note = "No hardware validation has been performed for these artifacts.";
  };
  source_files = {
    README = "${rockpro64FirmwareDir}/README.md";
    contract = "${rockpro64FirmwareDir}/contract.md";
    spi_layout = "${rockpro64FirmwareDir}/spi-layout.md";
    boot_cmd = "${rockpro64FirmwareDir}/boot.cmd";
    uboot_env = "${rockpro64FirmwareDir}/uboot.env";
    config_fragment = "${rockpro64FirmwareDir}/u-boot.config.fragment";
  };
  built_artifacts = {
    uboot = "${rockpro64Uboot}";
    spi_firmware = "${rockpro64SpiFirmware}";
    spi_installer_img = "${rockpro64SpiInstallerImg}";
    shared_disk_image_img = "${rockpro64SharedDiskImageImg}";
    release_manifest = "${rockpro64ReleaseManifest}";
  };
  provenance_note = "Generated from source scaffolding in firmware/rockpro64.";
}}
EOF
        '';

        rockpro64ReleaseBundle = pkgs.runCommand "rockpro64-release-bundle" { } ''
          mkdir -p "$out"

          cp ${rockpro64ReleaseManifest} "$out/manifest.json"
          cp ${rockpro64ReleaseManifest} "$out/rockpro64-release-manifest.json"
          cp ${rockpro64SpiInstallerImg} "$out/rockpro64-spi-installer.img"
          cp ${rockpro64SharedDiskImageImg} "$out/rockpro64-shared-disk-image.img"
          cp ${rockpro64ChecksumsProvenance}/provenance.json "$out/provenance.json"
          cp ${rockpro64ChecksumsProvenance}/checksums.txt "$out/checksums.txt"
        '';

        rockpro64ScaffoldCheck = pkgs.runCommand "rockpro64-scaffold-check" { } ''
          test -e ${rockpro64Uboot}/README.md
          test -e ${rockpro64SpiFirmware}/spi-layout.md
          test -s ${rockpro64ReleaseManifest}
          test -s ${rockpro64ChecksumsProvenance}/checksums.txt
          test -s ${rockpro64ChecksumsProvenance}/provenance.json
          mkdir -p "$out"
          printf '%s\n' "rockpro64 firmware scaffolding is wired up" > "$out/result"
        '';

        qemuArm64ToolingCheck = pkgs.runCommand "qemu-arm64-tooling-check" { } ''
          mkdir -p "$out"

          ${pkgs.qemu}/bin/qemu-system-aarch64 --version > "$out/qemu-version.txt"
          ${pkgs.qemu}/bin/qemu-system-aarch64 -machine help > "$out/machines.txt"
          ${pkgs.qemu}/bin/qemu-system-aarch64 -device help > "$out/devices.txt"

          grep -Eq '(^|[[:space:]])virt([[:space:]]|$)' "$out/machines.txt"
          grep -q 'qemu-xhci' "$out/devices.txt"
          grep -q 'usb-storage' "$out/devices.txt"
          grep -q 'nvme' "$out/devices.txt"
          grep -q 'virtio-blk-device' "$out/devices.txt"

          test -s ${validationDir}/qemu-arm64/generic-smoke.json
          test -s ${validationDir}/qemu-arm64/boot-smoke.json
          test -s ${validationDir}/rockpro64/lab-plan.json

          if grep -Eiq 'rockpro64|rk3399' "$out/machines.txt"; then
            printf '%s\n' \
              "QEMU lists a Rockchip-like machine, but bootswain still treats QEMU coverage as generic ARM64 only." \
              > "$out/rockchip-note.txt"
          else
            printf '%s\n' \
              "No rockpro64/rk3399 QEMU machine was found; QEMU coverage is generic ARM64 only." \
              > "$out/rockchip-note.txt"
          fi
        '';

        commonArgs = {
          inherit src;
          pname = "bootswain";
          version = "0.1.0";
          strictDeps = true;
          nativeBuildInputs = [
            pkgs.pkg-config
          ];
          buildInputs = [
            pkgs.udev
            pkgs.zstd
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        bootswain = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "-p bootswain-cli";
            meta = {
              description = "ROCKPro64-first host-side flash and probe tooling for stock U-Boot";
              license = with lib.licenses; [
                mit
                asl20
              ];
              mainProgram = "bootswain";
              platforms = lib.platforms.linux;
            };
          }
        );

        qemuArm64BootSmokeCheck = pkgs.runCommand "qemu-arm64-boot-smoke" { } ''
          mkdir -p "$out"

          ${bootswain}/bin/bootswain validate qemu-arm64 \
            --plan ${validationDir}/qemu-arm64/boot-smoke.json \
            --u-boot ${qemuArm64Uboot}/u-boot.bin \
            --qemu ${pkgs.qemu}/bin/qemu-system-aarch64 \
            --out "$out" \
            --json > "$out/stdout.json" 2> "$out/stderr.log"

          test -s "$out/validation-run.json"
          test -s "$out/qemu-command.json"
          test -s "$out/serial.log"
          test -e "$out/qemu-stderr.log"
          grep -q '"status": "passed"' "$out/validation-run.json"
          grep -q 'U-Boot' "$out/serial.log"
        '';

        flashApp = pkgs.writeShellApplication {
          name = "bootswain-flash";
          text = ''
            set -euo pipefail

            show_targets() {
              cat <<'EOF'
            available targets:
              rockpro64-spi-installer      artifact: spi-installer
              rockpro64-shared-disk-image  artifact: shared-disk-image
            EOF
            }

            usage() {
              cat <<'EOF'
            Usage:
              bootswain-flash --target <target> --device <device> [--dry-run] [--verify] [--verify-only] [--json] [--yes]
              bootswain-flash --list-targets

            Examples:
              nix run .#flash -- --target rockpro64-spi-installer --device /dev/sdX --dry-run
              nix run .#flash -- --target rockpro64-spi-installer --device /dev/sdX --verify --yes
            EOF
            }

            target=""
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
                --dry-run|--verify|--verify-only|--json|--yes)
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

            case "$target" in
              rockpro64-spi-installer)
                bundle="${rockpro64ReleaseBundle}"
                artifact="spi-installer"
                ;;
              rockpro64-shared-disk-image)
                bundle="${rockpro64ReleaseBundle}"
                artifact="shared-disk-image"
                ;;
              *)
                echo "error: unknown target: $target" >&2
                show_targets >&2
                exit 2
                ;;
            esac

            echo "bootswain flash target: $target" >&2
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
              --manifest "$bundle/manifest.json" \
              --artifact "$artifact" \
              --device "$device" \
              "''${passthrough[@]}"
          '';
        };

        flashAppWrapperCheck = pkgs.runCommand "flash-app-wrapper-check" { } ''
          mkdir -p "$out"

          ${flashApp}/bin/bootswain-flash --list-targets > "$out/targets.txt"
          grep -q 'rockpro64-spi-installer' "$out/targets.txt"
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

          set +e
          ${flashApp}/bin/bootswain-flash \
            --target rockpro64-spi-installer \
            --device /dev/null \
            --dry-run \
            > "$out/scaffold-refusal.out" 2> "$out/scaffold-refusal.err"
          status=$?
          set -e
          test "$status" -ne 0
          grep -q 'bootswain flash target: rockpro64-spi-installer' "$out/scaffold-refusal.err"
          grep -q 'bootswain flash artifact: spi-installer' "$out/scaffold-refusal.err"
          grep -q 'bootswain flash flags: --dry-run' "$out/scaffold-refusal.err"
          grep -q 'artifact spi-installer is not marked flashable' "$out/scaffold-refusal.err"

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
          grep -q 'artifact spi-installer is not marked flashable' "$out/passthrough.err"
        '';
      in
      {
        packages = {
          default = bootswain;
          inherit bootswain;
          "qemu-arm64-uboot" = qemuArm64Uboot;
          "rockpro64-uboot" = rockpro64Uboot;
          "rockpro64-spi-firmware" = rockpro64SpiFirmware;
          "rockpro64-spi-installer-img" = rockpro64SpiInstallerImg;
          "rockpro64-shared-disk-image-img" = rockpro64SharedDiskImageImg;
          "rockpro64-release-bundle" = rockpro64ReleaseBundle;
          "rockpro64-release-manifest" = rockpro64ReleaseManifest;
          "rockpro64-checksums-provenance" = rockpro64ChecksumsProvenance;
        };

        apps = {
          flash = {
            type = "app";
            program = "${flashApp}/bin/bootswain-flash";
            meta.description = "Build and flash a selected bootswain release artifact through bootswain flash sd";
          };
        };

        checks = {
          inherit bootswain;
          flash-app-wrapper = flashAppWrapperCheck;
          qemu-arm64-boot-smoke = qemuArm64BootSmokeCheck;
          qemu-arm64-tooling = qemuArm64ToolingCheck;
          rockpro64-scaffold = rockpro64ScaffoldCheck;
          workspace-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
            }
          );
          workspace-fmt = craneLib.cargoFmt {
            inherit src;
            pname = "bootswain";
          };
          workspace-test = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoExtraArgs = "--workspace";
            }
          );
        };

        devShells = rs-harbor.lib.mkDevShells {
          inherit pkgs cross;
          inherit (toolchain) craneLib;
          pkgConfigDeps = [
            pkgs.udev
            pkgs.zstd
          ];
          packages = [
            pkgs.git
            pkgs.qemu
            pkgs.zstd
          ];
        };
      }
    );
}
