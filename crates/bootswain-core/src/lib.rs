use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Board {
    RockPro64,
}

impl Board {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialResult {
    pub image_path: PathBuf,
    pub image_sha256: String,
    pub board: Board,
    pub serial_port: String,
    pub baud: u32,
    pub trial_number: u32,
    pub autoboot_result: AutobootResult,
    pub usb_tree_result: ProbeStepResult,
    pub usb_reset_result: ProbeStepResult,
    pub final_detected_usb_summary: String,
    pub raw_log_path: PathBuf,
}

impl TrialResult {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.autoboot_result != AutobootResult::Abort
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
