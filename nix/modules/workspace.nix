{
  pkgs,
  lib,
  craneLib,
  src,
  qemuArm64Uboot,
  validationDir,
  raspberryPi3BPlusReleaseBundle,
  raspberryPi3BPlusReleaseCandidate,
  rockpro64ExperimentalRelease,
  rockpro64ReleaseBundleExperimental,
}:
let
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
    test -s ${validationDir}/raspberrypi3bplus/lab-plan.json

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

  rockpro64ExperimentalBundleCheck =
    pkgs.runCommand "rockpro64-experimental-bundle-check" { } ''
      mkdir -p "$out"

      ${bootswain}/bin/bootswain firmware inspect \
        --manifest ${rockpro64ReleaseBundleExperimental}/release.json \
        > "$out/inspect.txt"
      ${bootswain}/bin/bootswain firmware artifacts \
        --manifest ${rockpro64ReleaseBundleExperimental}/release.json \
        > "$out/artifacts.txt"

      grep -q 'release: ${rockpro64ExperimentalRelease}' "$out/inspect.txt"
      grep -q 'artifacts: 6' "$out/inspect.txt"
      grep -q 'spi-installer: spi.installer.img' "$out/artifacts.txt"
      grep -q 'flashable: yes' "$out/artifacts.txt"
      grep -q 'u-boot-rockchip: u-boot-rockchip.bin' "$out/artifacts.txt"
      grep -q 'u-boot-rockchip-spi: u-boot-rockchip-spi.bin' "$out/artifacts.txt"
    '';
  raspberryPi3BPlusBundleCheck =
    pkgs.runCommand "raspberrypi3bplus-bundle-check" { } ''
      mkdir -p "$out"

      ${bootswain}/bin/bootswain firmware inspect \
        --manifest ${raspberryPi3BPlusReleaseBundle}/release.json \
        > "$out/inspect.txt"
      ${bootswain}/bin/bootswain firmware artifacts \
        --manifest ${raspberryPi3BPlusReleaseBundle}/release.json \
        > "$out/artifacts.txt"

      grep -q 'release: ${raspberryPi3BPlusReleaseCandidate}' "$out/inspect.txt"
      grep -q 'board: raspberry-pi-3-b-plus' "$out/inspect.txt"
      grep -q 'trusted-firmware-a: not-applicable' "$out/inspect.txt"
      grep -q 'spi-size-bytes: not-applicable' "$out/inspect.txt"
      grep -q 'artifacts: 3' "$out/inspect.txt"
      grep -q 'boot-partition-image: boot-partition.img' "$out/artifacts.txt"
      grep -q 'raspberry-pi-firmware: raspberry-pi-firmware.tar' "$out/artifacts.txt"
      grep -q 'u-boot-rpi3: u-boot-rpi3.bin' "$out/artifacts.txt"
      grep -q 'flashable: yes' "$out/artifacts.txt"
    '';
in
{
  inherit bootswain;

  packages = {
    default = bootswain;
    inherit bootswain;
  };

  checks = {
    inherit bootswain;
    qemu-arm64-boot-smoke = qemuArm64BootSmokeCheck;
    qemu-arm64-tooling = qemuArm64ToolingCheck;
    raspberrypi3bplus-bundle = raspberryPi3BPlusBundleCheck;
    rockpro64-experimental-bundle = rockpro64ExperimentalBundleCheck;
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
}
