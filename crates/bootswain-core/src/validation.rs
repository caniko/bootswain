use crate::{Board, BootProtocol, BootTarget, FirmwareArtifactKind};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

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
    Interrupted,
    NotRun,
    Skipped,
    Unsupported,
}

impl fmt::Display for ValidationOutcomeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => f.write_str("passed"),
            Self::Failed => f.write_str("failed"),
            Self::Interrupted => f.write_str("interrupted"),
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
