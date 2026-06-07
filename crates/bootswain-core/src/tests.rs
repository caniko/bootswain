
use crate::{
    BlockDeviceInfo, BlockDeviceKind, Board, BootProtocol, BootTarget, CompressionKind,
    FirmwareArtifact, FirmwareArtifactKind, FirmwareBootPolicy, FirmwareEnvironmentPolicy,
    FirmwareManifest, FirmwareSourceRevisions, FirmwareStorageLayout, FirmwareValidationClaim,
    FlashExecutionResult, FlashPlan, FlashRunResult, FlashStrategy, FlashVerificationResult,
    ImageInfo, QemuDiskInterface, QemuValidationConfig, ValidationExecutorKind, ValidationOutcome,
    ValidationOutcomeStatus, ValidationPlan, ValidationRun, ValidationScenario,
    ValidationScenarioKind, ValidationStep,
};
use std::path::PathBuf;

#[test]
fn public_enum_wire_names_are_stable() {
    let artifact_cases = [
        (FirmwareArtifactKind::SpiFirmware, "spi-firmware"),
        (FirmwareArtifactKind::SpiInstaller, "spi-installer"),
        (FirmwareArtifactKind::SharedDiskImage, "shared-disk-image"),
        (
            FirmwareArtifactKind::BootPartitionImage,
            "boot-partition-image",
        ),
        (
            FirmwareArtifactKind::RaspberryPiFirmware,
            "raspberry-pi-firmware",
        ),
        (FirmwareArtifactKind::Idbloader, "idbloader"),
        (FirmwareArtifactKind::UBootRpi3, "u-boot-rpi3"),
        (FirmwareArtifactKind::UBootItb, "u-boot-itb"),
        (FirmwareArtifactKind::UBootRockchip, "u-boot-rockchip"),
        (
            FirmwareArtifactKind::UBootRockchipSpi,
            "u-boot-rockchip-spi",
        ),
        (FirmwareArtifactKind::ReleaseManifest, "release-manifest"),
        (FirmwareArtifactKind::Checksums, "checksums"),
        (FirmwareArtifactKind::Provenance, "provenance"),
    ];
    for (kind, wire) in artifact_cases {
        assert_eq!(kind.to_string(), wire);
        assert_eq!(
            serde_json::to_string(&kind).expect("serialize kind"),
            format!("\"{wire}\"")
        );
        assert_eq!(
            wire.parse::<FirmwareArtifactKind>().expect("parse kind"),
            kind
        );
    }

    let boot_targets = [
        (BootTarget::Spi, "spi"),
        (BootTarget::Sd, "sd"),
        (BootTarget::Emmc, "emmc"),
        (BootTarget::Usb, "usb"),
        (BootTarget::Nvme, "nvme"),
        (BootTarget::Virtio, "virtio"),
    ];
    for (target, wire) in boot_targets {
        assert_eq!(target.to_string(), wire);
        assert_eq!(
            serde_json::to_string(&target).expect("serialize target"),
            format!("\"{wire}\"")
        );
    }

    let boot_protocols = [
        (BootProtocol::Uefi, "uefi"),
        (BootProtocol::Extlinux, "extlinux"),
        (BootProtocol::UBootScript, "u-boot-script"),
        (BootProtocol::UBootShell, "u-boot-shell"),
    ];
    for (protocol, wire) in boot_protocols {
        assert_eq!(protocol.to_string(), wire);
        assert_eq!(
            serde_json::to_string(&protocol).expect("serialize protocol"),
            format!("\"{wire}\"")
        );
    }

    assert_eq!(
        serde_json::to_string(&ValidationScenarioKind::StorageDiscovery)
            .expect("serialize scenario"),
        "\"storage-discovery\""
    );
    assert_eq!(
        serde_json::to_string(&ValidationScenarioKind::BootflowScan)
            .expect("serialize bootflow scenario"),
        "\"bootflow-scan\""
    );
    assert_eq!(
        serde_json::to_string(&ValidationExecutorKind::QemuArm64).expect("serialize executor"),
        "\"qemu-arm64\""
    );
    assert_eq!(
        Board::RaspberryPi3BPlus.to_string(),
        "raspberry-pi-3-b-plus"
    );
    assert_eq!(
        serde_json::to_string(&Board::RaspberryPi3BPlus).expect("serialize pi board"),
        "\"raspberry-pi-3-b-plus\""
    );
    assert_eq!(
        "rpi3bplus".parse::<Board>().expect("parse pi alias"),
        Board::RaspberryPi3BPlus
    );
    assert_eq!(
        serde_json::to_string(&ValidationExecutorKind::RockPro64Serial)
            .expect("serialize rockpro64 executor"),
        "\"rockpro64-serial\""
    );
    assert_eq!(
        "usb-storage"
            .parse::<QemuDiskInterface>()
            .expect("parse qemu disk"),
        QemuDiskInterface::UsbStorage
    );
    assert_eq!(ValidationOutcomeStatus::NotRun.to_string(), "not-run");
    assert_eq!(
        ValidationOutcomeStatus::Interrupted.to_string(),
        "interrupted"
    );
    assert_eq!(
        ValidationOutcomeStatus::Unsupported.to_string(),
        "unsupported"
    );
}

#[test]
fn firmware_manifest_round_trips() {
    let manifest = sample_manifest();

    let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    assert!(json.contains("\"board\": \"rock-pro64\""));
    assert!(json.contains("\"kind\": \"spi-installer\""));

    let decoded: FirmwareManifest = serde_json::from_str(&json).expect("decode manifest");
    assert_eq!(
        decoded.artifact(FirmwareArtifactKind::SpiInstaller),
        decoded.artifacts.first()
    );
    assert_eq!(
        decoded.artifact(FirmwareArtifactKind::SharedDiskImage),
        None
    );
}

#[test]
fn flash_run_result_serializes_execution_and_verification() {
    let run = FlashRunResult {
        execution: FlashExecutionResult {
            plan: sample_flash_plan(),
            bytes_written: Some(9),
        },
        verification: Some(FlashVerificationResult {
            image_path: PathBuf::from("spi.installer.img"),
            device_path: PathBuf::from("/dev/sdb"),
            strategy: FlashStrategy::RawCopy,
            verified_bytes: 9,
            matched: true,
        }),
    };

    let json = serde_json::to_string_pretty(&run).expect("serialize flash run");
    assert!(json.contains("\"bytes_written\": 9"));
    assert!(json.contains("\"matched\": true"));
    assert!(json.contains("\"strategy\": \"raw-copy\""));
}

#[test]
fn validation_plan_and_run_round_trip() {
    let plan = ValidationPlan {
        schema_version: 1,
        board: Board::GenericArm64Qemu,
        release: "2026.04-rockpro64.0".into(),
        executor: ValidationExecutorKind::QemuArm64,
        qemu: Some(QemuValidationConfig {
            machine: "virt".into(),
            cpu: Some("cortex-a57".into()),
            qemu_args: vec!["-nographic".into()],
            timeout_secs: 5,
        }),
        scenarios: vec![ValidationScenario {
            name: "prompt".into(),
            kind: ValidationScenarioKind::Prompt,
            target: Some(BootTarget::Virtio),
            protocol: Some(BootProtocol::UBootShell),
            artifact: Some(FirmwareArtifactKind::SpiFirmware),
            steps: vec![ValidationStep {
                name: "wait for prompt".into(),
                command: None,
                expect: vec!["=> ".into()],
                timeout_secs: 20,
            }],
        }],
    };
    let run = ValidationRun {
        plan,
        outcomes: vec![ValidationOutcome {
            scenario: "prompt".into(),
            status: ValidationOutcomeStatus::Passed,
            executor: Some(ValidationExecutorKind::QemuArm64),
            evidence: vec!["=> ".into()],
            logs: vec![PathBuf::from("serial.log")],
            failure: None,
        }],
    };

    let json = serde_json::to_string(&run).expect("serialize run");
    let decoded: ValidationRun = serde_json::from_str(&json).expect("decode run");
    assert!(decoded.passed());
    assert!(json.contains("u-boot-shell"));
}

#[test]
fn validation_run_requires_all_outcomes_to_pass() {
    let mut run = ValidationRun {
        plan: ValidationPlan {
            schema_version: 1,
            board: Board::RockPro64,
            release: "2026.04-rockpro64.0".into(),
            executor: ValidationExecutorKind::Deferred,
            qemu: None,
            scenarios: Vec::new(),
        },
        outcomes: vec![ValidationOutcome {
            scenario: "prompt".into(),
            status: ValidationOutcomeStatus::NotRun,
            executor: Some(ValidationExecutorKind::Deferred),
            evidence: Vec::new(),
            logs: Vec::new(),
            failure: Some("hardware pending".into()),
        }],
    };

    assert!(!run.passed());
    run.outcomes[0].status = ValidationOutcomeStatus::Failed;
    assert!(!run.passed());
    run.outcomes[0].status = ValidationOutcomeStatus::Passed;
    run.outcomes[0].failure = None;
    assert!(run.passed());
}

fn sample_manifest() -> FirmwareManifest {
    FirmwareManifest {
        schema_version: 1,
        release: "2026.04-rockpro64.0".into(),
        board: Board::RockPro64,
        sources: FirmwareSourceRevisions {
            u_boot: "v2026.04".into(),
            trusted_firmware_a: Some("v2.13.0".into()),
            bootswain: Some("dirty".into()),
            nixpkgs: Some("test-nixpkgs".into()),
        },
        storage_layout: FirmwareStorageLayout {
            spi_size_bytes: Some(16 * 1024 * 1024),
            spi_firmware_offset_bytes: Some(0),
            spi_firmware_size_bytes: None,
            environment_offset_bytes: None,
            environment_size_bytes: None,
            shared_storage_firmware_partition: Some("bootswain-firmware".into()),
        },
        boot_policy: FirmwareBootPolicy {
            default_order: vec![
                BootTarget::Emmc,
                BootTarget::Sd,
                BootTarget::Usb,
                BootTarget::Nvme,
            ],
            preferred_protocols: vec![BootProtocol::Uefi, BootProtocol::Extlinux],
            no_bootable_media_behavior: "show boot menu and serial diagnostics".into(),
        },
        environment_policy: FirmwareEnvironmentPolicy {
            persistent: false,
            location: None,
            stale_environment_safe: true,
            notes: Some("installer ignores saved environment".into()),
        },
        artifacts: vec![FirmwareArtifact {
            kind: FirmwareArtifactKind::SpiInstaller,
            path: PathBuf::from("spi.installer.img"),
            compression: CompressionKind::None,
            size_bytes: 1024,
            sha256: "deadbeef".into(),
            required_device_size_bytes: Some(2048),
            flashable: true,
        }],
        validation_claims: vec![FirmwareValidationClaim {
            target: BootTarget::Sd,
            protocols: vec![BootProtocol::Uefi],
            tested: false,
            scenarios: vec!["sd-uefi-boot".into()],
            notes: Some("lab pending".into()),
        }],
        unsupported_paths: vec!["phone/tablet button shortcuts".into()],
    }
}

fn sample_flash_plan() -> FlashPlan {
    FlashPlan {
        image: ImageInfo {
            path: PathBuf::from("spi.installer.img"),
            compression: CompressionKind::None,
            size_bytes: 9,
            sha256: "deadbeef".into(),
        },
        device: BlockDeviceInfo {
            original_path: PathBuf::from("/dev/disk/by-id/mock"),
            canonical_path: PathBuf::from("/dev/sdb"),
            block_name: "sdb".into(),
            kind: BlockDeviceKind::Disk,
            size_bytes: Some(16 * 1024 * 1024),
            removable: Some(true),
            model: Some("USB Reader".into()),
            vendor: Some("Kingston".into()),
        },
        strategy: FlashStrategy::RawCopy,
        dry_run: false,
    }
}
