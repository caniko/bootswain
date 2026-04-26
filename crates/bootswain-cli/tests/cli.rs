use assert_cmd::Command;
use bootswain_core::{Board, ValidationExecutorKind, ValidationPlan, ValidationScenarioKind};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn image_inspect_json_emits_machine_readable_output() {
    let tmp = tempdir().expect("tempdir");
    let image = tmp.path().join("artifact.img");
    fs::write(&image, b"bootswain").expect("write image");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "image",
            "inspect",
            "--image",
            image.to_str().expect("utf8 image path"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("\"compression\": \"none\""));
    assert!(stdout.contains("\"sha256\":"));
}

#[test]
fn firmware_inspect_reads_manifest_json() {
    let tmp = tempdir().expect("tempdir");
    let manifest = tmp.path().join("manifest.json");
    fs::write(&manifest, sample_manifest_json()).expect("write manifest");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "firmware",
            "inspect",
            "--manifest",
            manifest.to_str().expect("utf8 manifest path"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("release: 2026.04-rockpro64.0"));
    assert!(stdout.contains("artifacts: 1"));
}

#[test]
fn firmware_artifacts_json_lists_artifact_kinds() {
    let tmp = tempdir().expect("tempdir");
    let manifest = tmp.path().join("manifest.json");
    fs::write(&manifest, sample_manifest_json()).expect("write manifest");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "firmware",
            "artifacts",
            "--manifest",
            manifest.to_str().expect("utf8 manifest path"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("\"kind\": \"spi-installer\""));
}

#[test]
fn firmware_artifacts_human_lists_flashable_state() {
    let tmp = tempdir().expect("tempdir");
    let manifest = tmp.path().join("manifest.json");
    fs::write(&manifest, sample_manifest_json()).expect("write manifest");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "firmware",
            "artifacts",
            "--manifest",
            manifest.to_str().expect("utf8 manifest path"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("spi-installer: spi.installer.img"));
    assert!(stdout.contains("flashable: yes"));
}

#[test]
fn validate_run_materializes_not_run_report() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("validation-plan.json");
    let out = tmp.path().join("validation-out");
    fs::write(&plan, sample_validation_plan_json()).expect("write plan");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "run",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--out",
            out.to_str().expect("utf8 out path"),
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("\"status\": \"not-run\""));
    assert!(out.join("validation-run.json").exists());
}

#[test]
fn validate_run_human_output_marks_scenarios_not_run() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("validation-plan.json");
    let out = tmp.path().join("validation-out");
    fs::write(&plan, sample_validation_plan_json()).expect("write plan");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "run",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--out",
            out.to_str().expect("utf8 out path"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("Validation run"));
    assert!(stdout.contains("prompt: not-run"));
}

#[test]
fn validate_qemu_arm64_dry_run_writes_stable_outputs() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("qemu-plan.json");
    let u_boot = tmp.path().join("u-boot.bin");
    let out = tmp.path().join("qemu-out");
    fs::write(&plan, sample_qemu_validation_plan_json()).expect("write qemu plan");
    fs::write(&u_boot, b"fake u-boot").expect("write fake u-boot");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "qemu-arm64",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--u-boot",
            u_boot.to_str().expect("utf8 u-boot path"),
            "--out",
            out.to_str().expect("utf8 out path"),
            "--dry-run",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).expect("stdout utf8");
    assert!(stdout.contains("\"executor\": \"qemu-arm64\""));
    assert!(stdout.contains("\"status\": \"not-run\""));
    assert!(out.join("validation-run.json").exists());
    assert!(out.join("serial.log").exists());
    assert!(out.join("qemu-command.json").exists());
}

#[test]
fn validate_qemu_arm64_requires_existing_uboot_binary() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("qemu-plan.json");
    fs::write(&plan, sample_qemu_validation_plan_json()).expect("write qemu plan");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "qemu-arm64",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--u-boot",
            tmp.path()
                .join("missing-u-boot.bin")
                .to_str()
                .expect("utf8 u-boot path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("U-Boot binary does not exist"));
}

#[test]
fn validate_qemu_arm64_rejects_rockpro64_lab_plan() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("rockpro64-plan.json");
    let u_boot = tmp.path().join("u-boot.bin");
    fs::write(&plan, sample_validation_plan_json()).expect("write rockpro64 plan");
    fs::write(&u_boot, b"fake u-boot").expect("write fake u-boot");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "qemu-arm64",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--u-boot",
            u_boot.to_str().expect("utf8 u-boot path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("ROCKPro64 claims require hardware"));
}

#[test]
fn validate_rockpro64_serial_rejects_missing_scenario_before_port_open() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("rockpro64-serial-plan.json");
    fs::write(&plan, sample_rockpro64_serial_plan_json()).expect("write rockpro64 serial plan");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "rockpro64-serial",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--port",
            "/dev/bootswain-missing-tty",
            "--scenario",
            "missing",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("does not contain scenario"));
}

#[test]
fn validate_rockpro64_serial_requires_serial_executor_before_port_open() {
    let tmp = tempdir().expect("tempdir");
    let plan = tmp.path().join("rockpro64-plan.json");
    fs::write(&plan, sample_validation_plan_json()).expect("write rockpro64 plan");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "validate",
            "rockpro64-serial",
            "--plan",
            plan.to_str().expect("utf8 plan path"),
            "--port",
            "/dev/bootswain-missing-tty",
            "--scenario",
            "prompt",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("requires executor rockpro64-serial"));
}

#[test]
fn qemu_fixture_is_generic_arm64_only() {
    let plan = load_repo_validation_plan("validation/qemu-arm64/generic-smoke.json");
    let boot_smoke = load_repo_validation_plan("validation/qemu-arm64/boot-smoke.json");

    assert_eq!(plan.board, Board::GenericArm64Qemu);
    assert_eq!(plan.executor, ValidationExecutorKind::QemuArm64);
    assert_eq!(boot_smoke.board, Board::GenericArm64Qemu);
    assert_eq!(boot_smoke.executor, ValidationExecutorKind::QemuArm64);
    assert_eq!(boot_smoke.scenarios.len(), 1);
    assert!(
        plan.scenarios
            .iter()
            .any(|scenario| scenario.kind == ValidationScenarioKind::BootflowScan)
    );
    assert!(
        plan.scenarios
            .iter()
            .any(|scenario| scenario.name == "generic-virtio-discovery")
    );
}

#[test]
fn rockpro64_lab_fixture_covers_release_claim_scenarios() {
    let plan = load_repo_validation_plan("validation/rockpro64/lab-plan.json");
    let required = [
        "spi-install",
        "spi-reinstall",
        "spi-erase",
        "spi-post-install-prompt",
        "sd-uefi-boot",
        "sd-extlinux-boot",
        "emmc-uefi-boot",
        "emmc-extlinux-boot",
        "usb-uefi-boot",
        "usb-extlinux-boot",
        "nvme-uefi-boot",
        "nvme-extlinux-boot",
        "no-bootable-media-ui",
        "recovery-console",
    ];

    assert_eq!(plan.board, Board::RockPro64);
    assert_eq!(plan.executor, ValidationExecutorKind::RockPro64Serial);
    for name in required {
        assert!(
            plan.scenarios.iter().any(|scenario| scenario.name == name),
            "missing required scenario {name}"
        );
    }
}

#[test]
fn flash_artifact_requires_manifest() {
    let tmp = tempdir().expect("tempdir");
    let device = tmp.path().join("fake-device");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "flash",
            "sd",
            "--artifact",
            "spi-installer",
            "--device",
            device.to_str().expect("utf8 device path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("--artifact requires --manifest"));
}

#[test]
fn flash_manifest_requires_artifact() {
    let tmp = tempdir().expect("tempdir");
    let manifest = tmp.path().join("manifest.json");
    let device = tmp.path().join("fake-device");
    fs::write(&manifest, sample_manifest_json()).expect("write manifest");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "flash",
            "sd",
            "--manifest",
            manifest.to_str().expect("utf8 manifest path"),
            "--device",
            device.to_str().expect("utf8 device path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("--manifest requires --artifact"));
}

#[test]
fn flash_refuses_non_flashable_manifest_artifacts_before_device_access() {
    let tmp = tempdir().expect("tempdir");
    let manifest = tmp.path().join("manifest.json");
    let device = tmp.path().join("fake-device");
    fs::write(&manifest, sample_non_flashable_manifest_json()).expect("write manifest");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "flash",
            "sd",
            "--manifest",
            manifest.to_str().expect("utf8 manifest path"),
            "--artifact",
            "spi-installer",
            "--device",
            device.to_str().expect("utf8 device path"),
            "--dry-run",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(stderr.contains("is not marked flashable"));
}

#[test]
fn flash_allows_non_flashable_manifest_artifacts_with_explicit_override() {
    let tmp = tempdir().expect("tempdir");
    let manifest = tmp.path().join("manifest.json");
    let image = tmp.path().join("spi.installer.img");
    let device = tmp.path().join("fake-device");
    fs::write(&manifest, sample_non_flashable_manifest_json()).expect("write manifest");
    fs::write(&image, b"bootswain").expect("write image");
    fs::write(&device, b"not a block device").expect("write fake device");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "flash",
            "sd",
            "--manifest",
            manifest.to_str().expect("utf8 manifest path"),
            "--artifact",
            "spi-installer",
            "--device",
            device.to_str().expect("utf8 device path"),
            "--dry-run",
            "--allow-non-flashable",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(!stderr.contains("is not marked flashable"));
    assert!(stderr.contains("target is not a block device"));
}

#[test]
fn flash_accepts_non_flashable_override_without_manifest_artifact() {
    let tmp = tempdir().expect("tempdir");
    let image = tmp.path().join("image.img");
    let device = tmp.path().join("fake-device");
    fs::write(&image, b"bootswain").expect("write image");
    fs::write(&device, b"not a block device").expect("write fake device");

    let output = Command::cargo_bin("bootswain")
        .expect("bootswain binary")
        .args([
            "flash",
            "sd",
            "--image",
            image.to_str().expect("utf8 image path"),
            "--device",
            device.to_str().expect("utf8 device path"),
            "--dry-run",
            "--allow-non-flashable",
        ])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr utf8");
    assert!(!stderr.contains("is not marked flashable"));
    assert!(stderr.contains("target is not a block device"));
}

fn sample_manifest_json() -> &'static str {
    r#"{
  "schema_version": 1,
  "release": "2026.04-rockpro64.0",
  "board": "rock-pro64",
  "sources": {
    "u_boot": "v2026.04",
    "trusted_firmware_a": "v2.13.0",
    "bootswain": "test",
    "nixpkgs": "test-nixpkgs"
  },
  "storage_layout": {
    "spi_size_bytes": 16777216,
    "spi_firmware_offset_bytes": 0,
    "spi_firmware_size_bytes": null,
    "environment_offset_bytes": null,
    "environment_size_bytes": null,
    "shared_storage_firmware_partition": "bootswain-firmware"
  },
  "boot_policy": {
    "default_order": ["emmc", "sd", "usb", "nvme"],
    "preferred_protocols": ["uefi", "extlinux"],
    "no_bootable_media_behavior": "show boot menu"
  },
  "environment_policy": {
    "persistent": false,
    "location": null,
    "stale_environment_safe": true,
    "notes": "test"
  },
  "artifacts": [
    {
      "kind": "spi-installer",
      "path": "spi.installer.img",
      "compression": "none",
      "size_bytes": 9,
      "sha256": "deadbeef",
      "required_device_size_bytes": 9,
      "flashable": true
    }
  ],
  "validation_claims": [
    {
      "target": "sd",
      "protocols": ["u-boot-shell"],
      "tested": false,
      "scenarios": ["sd-installer-menu"],
      "notes": "lab pending"
    }
  ],
  "unsupported_paths": ["phone shortcuts"]
}"#
}

fn load_repo_validation_plan(path: &str) -> ValidationPlan {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let content = fs::read_to_string(repo_root.join(path)).expect("read validation fixture");
    serde_json::from_str(&content).expect("parse validation fixture")
}

fn sample_non_flashable_manifest_json() -> &'static str {
    r#"{
  "schema_version": 1,
  "release": "2026.04-rockpro64.0",
  "board": "rock-pro64",
  "sources": {
    "u_boot": "v2026.04",
    "trusted_firmware_a": "v2.13.0",
    "bootswain": "test",
    "nixpkgs": "test-nixpkgs"
  },
  "storage_layout": {
    "spi_size_bytes": 16777216,
    "spi_firmware_offset_bytes": 0,
    "spi_firmware_size_bytes": null,
    "environment_offset_bytes": null,
    "environment_size_bytes": null,
    "shared_storage_firmware_partition": "bootswain-firmware"
  },
  "boot_policy": {
    "default_order": ["sd"],
    "preferred_protocols": ["u-boot-shell"],
    "no_bootable_media_behavior": "show boot menu"
  },
  "environment_policy": {
    "persistent": false,
    "location": null,
    "stale_environment_safe": true,
    "notes": "test"
  },
  "artifacts": [
    {
      "kind": "spi-installer",
      "path": "spi.installer.img",
      "compression": "none",
      "size_bytes": 9,
      "sha256": "0c8bb5af83b12eb17b5bcdad45b7ea18fa2653749afa92c67a33fdd4ede619c1",
      "required_device_size_bytes": 9,
      "flashable": false
    }
  ],
  "validation_claims": [],
  "unsupported_paths": []
}"#
}

fn sample_validation_plan_json() -> &'static str {
    r#"{
  "schema_version": 1,
  "board": "rock-pro64",
  "release": "2026.04-rockpro64.0",
  "scenarios": [
    {
      "name": "prompt",
      "kind": "prompt",
      "target": "spi",
      "protocol": "u-boot-shell",
      "artifact": "spi-firmware",
      "steps": [
        {
          "name": "wait for prompt",
          "command": null,
          "expect": ["=> "],
          "timeout_secs": 20
        }
      ]
    }
  ]
}"#
}

fn sample_qemu_validation_plan_json() -> &'static str {
    r#"{
  "schema_version": 1,
  "board": "generic-arm64-qemu",
  "release": "qemu-arm64-smoke.0",
  "executor": "qemu-arm64",
  "qemu": {
    "machine": "virt",
    "cpu": "cortex-a57",
    "qemu_args": ["-machine", "virt", "-nographic"],
    "timeout_secs": 5
  },
  "scenarios": [
    {
      "name": "generic-u-boot-prompt",
      "kind": "prompt",
      "target": "virtio",
      "protocol": "u-boot-shell",
      "artifact": null,
      "steps": [
        {
          "name": "wait for prompt",
          "command": null,
          "expect": ["=> "],
          "timeout_secs": 5
        }
      ]
    }
  ]
}"#
}

fn sample_rockpro64_serial_plan_json() -> &'static str {
    r#"{
  "schema_version": 1,
  "board": "rock-pro64",
  "release": "2026.04-rockpro64.0",
  "executor": "rockpro64-serial",
  "qemu": null,
  "scenarios": [
    {
      "name": "recovery-console",
      "kind": "prompt",
      "target": "spi",
      "protocol": "u-boot-shell",
      "artifact": "spi-firmware",
      "steps": [
        {
          "name": "wait for prompt",
          "command": null,
          "expect": ["=> "],
          "timeout_secs": 20
        }
      ]
    }
  ]
}"#
}
