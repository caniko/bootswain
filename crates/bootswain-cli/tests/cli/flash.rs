use super::support::{sample_manifest_json, sample_non_flashable_manifest_json};
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

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
