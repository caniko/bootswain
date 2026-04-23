use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Board {
    GenericArm64Qemu,
    RockPro64,
}

impl Board {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GenericArm64Qemu => "generic-arm64-qemu",
            Self::RockPro64 => "rockpro64",
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompressionKind {
    None,
    Zstd,
}

impl fmt::Display for CompressionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Zstd => f.write_str("zstd"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageInfo {
    pub path: PathBuf,
    pub compression: CompressionKind,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FirmwareArtifactKind {
    SpiFirmware,
    SpiInstaller,
    SharedDiskImage,
    Idbloader,
    UBootItb,
    UBootRockchip,
    UBootRockchipSpi,
    ReleaseManifest,
    Checksums,
    Provenance,
}

impl FirmwareArtifactKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SpiFirmware => "spi-firmware",
            Self::SpiInstaller => "spi-installer",
            Self::SharedDiskImage => "shared-disk-image",
            Self::Idbloader => "idbloader",
            Self::UBootItb => "u-boot-itb",
            Self::UBootRockchip => "u-boot-rockchip",
            Self::UBootRockchipSpi => "u-boot-rockchip-spi",
            Self::ReleaseManifest => "release-manifest",
            Self::Checksums => "checksums",
            Self::Provenance => "provenance",
        }
    }
}

impl fmt::Display for FirmwareArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FirmwareArtifactKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "spi-firmware" => Ok(Self::SpiFirmware),
            "spi-installer" => Ok(Self::SpiInstaller),
            "shared-disk-image" => Ok(Self::SharedDiskImage),
            "idbloader" => Ok(Self::Idbloader),
            "u-boot-itb" => Ok(Self::UBootItb),
            "u-boot-rockchip" => Ok(Self::UBootRockchip),
            "u-boot-rockchip-spi" => Ok(Self::UBootRockchipSpi),
            "release-manifest" => Ok(Self::ReleaseManifest),
            "checksums" => Ok(Self::Checksums),
            "provenance" => Ok(Self::Provenance),
            _ => Err(format!("unknown firmware artifact kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootProtocol {
    Uefi,
    Extlinux,
    UBootScript,
    UBootShell,
}

impl fmt::Display for BootProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uefi => f.write_str("uefi"),
            Self::Extlinux => f.write_str("extlinux"),
            Self::UBootScript => f.write_str("u-boot-script"),
            Self::UBootShell => f.write_str("u-boot-shell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootTarget {
    Spi,
    Sd,
    Emmc,
    Usb,
    Nvme,
    Virtio,
}

impl fmt::Display for BootTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spi => f.write_str("spi"),
            Self::Sd => f.write_str("sd"),
            Self::Emmc => f.write_str("emmc"),
            Self::Usb => f.write_str("usb"),
            Self::Nvme => f.write_str("nvme"),
            Self::Virtio => f.write_str("virtio"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareSourceRevisions {
    pub u_boot: String,
    pub trusted_firmware_a: String,
    pub bootswain: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareStorageLayout {
    pub spi_size_bytes: u64,
    pub spi_firmware_offset_bytes: u64,
    pub spi_firmware_size_bytes: Option<u64>,
    pub environment_offset_bytes: Option<u64>,
    pub environment_size_bytes: Option<u64>,
    pub shared_storage_firmware_partition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareBootPolicy {
    pub default_order: Vec<BootTarget>,
    pub preferred_protocols: Vec<BootProtocol>,
    pub no_bootable_media_behavior: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareEnvironmentPolicy {
    pub persistent: bool,
    pub location: Option<String>,
    pub stale_environment_safe: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareArtifact {
    pub kind: FirmwareArtifactKind,
    pub path: PathBuf,
    pub compression: CompressionKind,
    pub size_bytes: u64,
    pub sha256: String,
    pub required_device_size_bytes: Option<u64>,
    pub flashable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareValidationClaim {
    pub target: BootTarget,
    pub protocols: Vec<BootProtocol>,
    pub tested: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareManifest {
    pub schema_version: u32,
    pub release: String,
    pub board: Board,
    pub sources: FirmwareSourceRevisions,
    pub storage_layout: FirmwareStorageLayout,
    pub boot_policy: FirmwareBootPolicy,
    pub environment_policy: FirmwareEnvironmentPolicy,
    pub artifacts: Vec<FirmwareArtifact>,
    pub validation_claims: Vec<FirmwareValidationClaim>,
    pub unsupported_paths: Vec<String>,
}

impl FirmwareManifest {
    #[must_use]
    pub fn artifact(&self, kind: FirmwareArtifactKind) -> Option<&FirmwareArtifact> {
        self.artifacts.iter().find(|artifact| artifact.kind == kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockDeviceKind {
    Disk,
    Loop,
    Ram,
    DeviceMapper,
    MdRaid,
    Unknown,
}

impl fmt::Display for BlockDeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disk => f.write_str("disk"),
            Self::Loop => f.write_str("loop"),
            Self::Ram => f.write_str("ram"),
            Self::DeviceMapper => f.write_str("device-mapper"),
            Self::MdRaid => f.write_str("md-raid"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDeviceInfo {
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub block_name: String,
    pub kind: BlockDeviceKind,
    pub size_bytes: Option<u64>,
    pub removable: Option<bool>,
    pub model: Option<String>,
    pub vendor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlashStrategy {
    RawCopy,
    ZstdDecompress,
}

impl fmt::Display for FlashStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RawCopy => f.write_str("raw-copy"),
            Self::ZstdDecompress => f.write_str("zstd-decompress"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashPlan {
    pub image: ImageInfo,
    pub device: BlockDeviceInfo,
    pub strategy: FlashStrategy,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashExecutionResult {
    pub plan: FlashPlan,
    pub bytes_written: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashVerificationResult {
    pub image_path: PathBuf,
    pub device_path: PathBuf,
    pub strategy: FlashStrategy,
    pub verified_bytes: u64,
    pub matched: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashRunResult {
    pub execution: FlashExecutionResult,
    pub verification: Option<FlashVerificationResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutobootResult {
    Abort,
    WarnEnumerate,
    NoDevice,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProbeStepResult {
    Ok,
    Reset,
    Timeout,
    NotRun,
}

impl fmt::Display for ProbeStepResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => f.write_str("ok"),
            Self::Reset => f.write_str("reset"),
            Self::Timeout => f.write_str("timeout"),
            Self::NotRun => f.write_str("not-run"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureStage {
    None,
    Autoboot,
    UsbStart,
    UsbTree1,
    UsbReset,
    UsbTree2,
}

impl fmt::Display for FailureStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Autoboot => f.write_str("autoboot"),
            Self::UsbStart => f.write_str("usb-start"),
            Self::UsbTree1 => f.write_str("usb-tree-1"),
            Self::UsbReset => f.write_str("usb-reset"),
            Self::UsbTree2 => f.write_str("usb-tree-2"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialResult {
    pub image_path: PathBuf,
    pub image_sha256: String,
    pub board: Board,
    pub serial_port: String,
    pub baud: u32,
    pub trial_number: u32,
    pub autoboot_result: AutobootResult,
    pub usb_start_result: ProbeStepResult,
    pub usb_tree_result: ProbeStepResult,
    pub usb_reset_result: ProbeStepResult,
    pub failure_stage: FailureStage,
    pub final_detected_usb_summary: String,
    pub raw_log_path: PathBuf,
}

impl TrialResult {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failure_stage == FailureStage::None
            && self.usb_start_result == ProbeStepResult::Ok
            && self.usb_tree_result == ProbeStepResult::Ok
            && self.usb_reset_result == ProbeStepResult::Ok
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryResult {
    pub image_path: PathBuf,
    pub image_sha256: String,
    pub board: Board,
    pub total_trials: u32,
    pub clean_trials: u32,
    pub failing_trials: u32,
    pub trial_results: Vec<TrialResult>,
}

impl SummaryResult {
    #[must_use]
    pub fn from_trials(image: &ImageInfo, board: Board, trial_results: Vec<TrialResult>) -> Self {
        let total_trials = trial_results.len() as u32;
        let clean_trials = trial_results
            .iter()
            .filter(|trial| trial.is_clean())
            .count() as u32;
        let failing_trials = total_trials.saturating_sub(clean_trials);

        Self {
            image_path: image.path.clone(),
            image_sha256: image.sha256.clone(),
            board,
            total_trials,
            clean_trials,
            failing_trials,
            trial_results,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationScenarioKind {
    Prompt,
    UsbProbe,
    SpiInstall,
    SpiErase,
    BootMenu,
    BootflowScan,
    UefiBoot,
    ExtlinuxBoot,
    StorageDiscovery,
}

impl fmt::Display for ValidationScenarioKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prompt => f.write_str("prompt"),
            Self::UsbProbe => f.write_str("usb-probe"),
            Self::SpiInstall => f.write_str("spi-install"),
            Self::SpiErase => f.write_str("spi-erase"),
            Self::BootMenu => f.write_str("boot-menu"),
            Self::BootflowScan => f.write_str("bootflow-scan"),
            Self::UefiBoot => f.write_str("uefi-boot"),
            Self::ExtlinuxBoot => f.write_str("extlinux-boot"),
            Self::StorageDiscovery => f.write_str("storage-discovery"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationExecutorKind {
    #[default]
    Deferred,
    QemuArm64,
    #[serde(rename = "rockpro64-serial")]
    RockPro64Serial,
}

impl fmt::Display for ValidationExecutorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deferred => f.write_str("deferred"),
            Self::QemuArm64 => f.write_str("qemu-arm64"),
            Self::RockPro64Serial => f.write_str("rockpro64-serial"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QemuDiskInterface {
    Virtio,
    UsbStorage,
    Nvme,
}

impl fmt::Display for QemuDiskInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Virtio => f.write_str("virtio"),
            Self::UsbStorage => f.write_str("usb-storage"),
            Self::Nvme => f.write_str("nvme"),
        }
    }
}

impl FromStr for QemuDiskInterface {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "virtio" => Ok(Self::Virtio),
            "usb-storage" => Ok(Self::UsbStorage),
            "nvme" => Ok(Self::Nvme),
            _ => Err(format!("unknown QEMU disk interface: {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QemuValidationConfig {
    pub machine: String,
    #[serde(default)]
    pub cpu: Option<String>,
    pub qemu_args: Vec<String>,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationStep {
    pub name: String,
    pub command: Option<String>,
    pub expect: Vec<String>,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationScenario {
    pub name: String,
    pub kind: ValidationScenarioKind,
    pub target: Option<BootTarget>,
    pub protocol: Option<BootProtocol>,
    pub artifact: Option<FirmwareArtifactKind>,
    pub steps: Vec<ValidationStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationPlan {
    pub schema_version: u32,
    pub board: Board,
    pub release: String,
    #[serde(default)]
    pub executor: ValidationExecutorKind,
    #[serde(default)]
    pub qemu: Option<QemuValidationConfig>,
    pub scenarios: Vec<ValidationScenario>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationOutcomeStatus {
    Passed,
    Failed,
    NotRun,
    Skipped,
    Unsupported,
}

impl fmt::Display for ValidationOutcomeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => f.write_str("passed"),
            Self::Failed => f.write_str("failed"),
            Self::NotRun => f.write_str("not-run"),
            Self::Skipped => f.write_str("skipped"),
            Self::Unsupported => f.write_str("unsupported"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationOutcome {
    pub scenario: String,
    pub status: ValidationOutcomeStatus,
    #[serde(default)]
    pub executor: Option<ValidationExecutorKind>,
    pub evidence: Vec<String>,
    #[serde(default)]
    pub logs: Vec<PathBuf>,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationRun {
    pub plan: ValidationPlan,
    pub outcomes: Vec<ValidationOutcome>,
}

impl ValidationRun {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| outcome.status == ValidationOutcomeStatus::Passed)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlockDeviceInfo, BlockDeviceKind, Board, BootProtocol, BootTarget, CompressionKind,
        FirmwareArtifact, FirmwareArtifactKind, FirmwareBootPolicy, FirmwareEnvironmentPolicy,
        FirmwareManifest, FirmwareSourceRevisions, FirmwareStorageLayout, FirmwareValidationClaim,
        FlashExecutionResult, FlashRunResult, FlashStrategy, FlashVerificationResult,
        QemuDiskInterface, QemuValidationConfig, ValidationExecutorKind, ValidationOutcome,
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
            (FirmwareArtifactKind::Idbloader, "idbloader"),
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
                trusted_firmware_a: "v2.13.0".into(),
                bootswain: Some("dirty".into()),
            },
            storage_layout: FirmwareStorageLayout {
                spi_size_bytes: 16 * 1024 * 1024,
                spi_firmware_offset_bytes: 0,
                spi_firmware_size_bytes: None,
                environment_offset_bytes: None,
                environment_size_bytes: None,
                shared_storage_firmware_partition: Some("bootswain-firmware".into()),
            },
            boot_policy: FirmwareBootPolicy {
                default_order: vec![BootTarget::Sd, BootTarget::Emmc, BootTarget::Usb],
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
                notes: Some("lab pending".into()),
            }],
            unsupported_paths: vec!["phone/tablet button shortcuts".into()],
        }
    }

    fn sample_flash_plan() -> super::FlashPlan {
        super::FlashPlan {
            image: super::ImageInfo {
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
}
