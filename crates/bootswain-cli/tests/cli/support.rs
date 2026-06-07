use bootswain_core::ValidationPlan;
use std::fs;
use std::path::PathBuf;

pub(crate) fn sample_manifest_json() -> &'static str {
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

pub(crate) fn load_repo_validation_plan(path: &str) -> ValidationPlan {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let content = fs::read_to_string(repo_root.join(path)).expect("read validation fixture");
    serde_json::from_str(&content).expect("parse validation fixture")
}

pub(crate) fn sample_non_flashable_manifest_json() -> &'static str {
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

pub(crate) fn sample_validation_plan_json() -> &'static str {
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

pub(crate) fn sample_qemu_validation_plan_json() -> &'static str {
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

pub(crate) fn sample_rockpro64_serial_plan_json() -> &'static str {
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
