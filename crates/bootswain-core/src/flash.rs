use crate::ImageInfo;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

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
