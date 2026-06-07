use crate::Board;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

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
    BootPartitionImage,
    RaspberryPiFirmware,
    Idbloader,
    UBootRpi3,
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
            Self::BootPartitionImage => "boot-partition-image",
            Self::RaspberryPiFirmware => "raspberry-pi-firmware",
            Self::Idbloader => "idbloader",
            Self::UBootRpi3 => "u-boot-rpi3",
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
            "boot-partition-image" => Ok(Self::BootPartitionImage),
            "raspberry-pi-firmware" => Ok(Self::RaspberryPiFirmware),
            "idbloader" => Ok(Self::Idbloader),
            "u-boot-rpi3" => Ok(Self::UBootRpi3),
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
    #[serde(default)]
    pub trusted_firmware_a: Option<String>,
    pub bootswain: Option<String>,
    #[serde(default)]
    pub nixpkgs: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareStorageLayout {
    #[serde(default)]
    pub spi_size_bytes: Option<u64>,
    #[serde(default)]
    pub spi_firmware_offset_bytes: Option<u64>,
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
    #[serde(default)]
    pub scenarios: Vec<String>,
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
