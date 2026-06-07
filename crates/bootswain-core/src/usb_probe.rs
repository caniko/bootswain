use crate::{Board, ImageInfo, read_json_file, write_pretty_json_file};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbProbeValidationPlan {
    pub name: String,
    #[serde(with = "board_wire")]
    pub board: Board,
    pub image: ImageInfo,
    pub scenarios: Vec<UsbProbeValidationScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbProbeValidationScenario {
    pub name: String,
    pub description: String,
    pub expected: UsbProbeValidationExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UsbProbeValidationExpectation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autoboot_result: Option<AutobootResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_stage: Option<FailureStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb_start_result: Option<ProbeStepResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb_tree_result: Option<ProbeStepResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb_reset_result: Option<ProbeStepResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_detected_usb_summary_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clean: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsbProbeValidationStatus {
    Passed,
    Failed,
}

impl fmt::Display for UsbProbeValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => f.write_str("passed"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbProbeValidationOutcome {
    pub scenario_name: String,
    pub status: UsbProbeValidationStatus,
    pub mismatches: Vec<String>,
    pub trial: TrialResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsbProbeValidationReport {
    pub plan: UsbProbeValidationPlan,
    pub outcomes: Vec<UsbProbeValidationOutcome>,
}

impl UsbProbeValidationPlan {
    #[must_use]
    pub fn rockpro64_usb_validation_plan(image: ImageInfo) -> Self {
        Self {
            name: "rockpro64-usb-validation".to_owned(),
            board: Board::RockPro64,
            image,
            scenarios: vec![
                UsbProbeValidationScenario::clean_usb_sequence(),
                UsbProbeValidationScenario::autoboot_reset(),
                UsbProbeValidationScenario::usb_start_reset(),
                UsbProbeValidationScenario::usb_start_timeout(),
                UsbProbeValidationScenario::usb_tree_reset(),
                UsbProbeValidationScenario::usb_reset_reset(),
            ],
        }
    }

    #[must_use]
    pub fn scenario(&self, name: &str) -> Option<&UsbProbeValidationScenario> {
        self.scenarios.iter().find(|scenario| scenario.name == name)
    }
}

impl UsbProbeValidationScenario {
    #[must_use]
    pub fn clean_usb_sequence() -> Self {
        Self {
            name: "clean-usb-sequence".to_owned(),
            description: "U-Boot prompt is reached and the full USB probe sequence succeeds."
                .to_owned(),
            expected: UsbProbeValidationExpectation {
                autoboot_result: Some(AutobootResult::Other),
                failure_stage: Some(FailureStage::None),
                usb_start_result: Some(ProbeStepResult::Ok),
                usb_tree_result: Some(ProbeStepResult::Ok),
                usb_reset_result: Some(ProbeStepResult::Ok),
                final_detected_usb_summary_contains: Some("Mass Storage".to_owned()),
                clean: Some(true),
            },
        }
    }

    #[must_use]
    pub fn autoboot_reset() -> Self {
        Self {
            name: "autoboot-reset".to_owned(),
            description: "The board resets before the prompt appears.".to_owned(),
            expected: UsbProbeValidationExpectation {
                autoboot_result: Some(AutobootResult::Abort),
                failure_stage: Some(FailureStage::Autoboot),
                usb_start_result: Some(ProbeStepResult::NotRun),
                usb_tree_result: Some(ProbeStepResult::NotRun),
                usb_reset_result: Some(ProbeStepResult::NotRun),
                final_detected_usb_summary_contains: None,
                clean: Some(false),
            },
        }
    }

    #[must_use]
    pub fn usb_start_reset() -> Self {
        Self {
            name: "usb-start-reset".to_owned(),
            description: "usb start triggers a reset before any USB tree output appears."
                .to_owned(),
            expected: UsbProbeValidationExpectation {
                autoboot_result: Some(AutobootResult::Other),
                failure_stage: Some(FailureStage::UsbStart),
                usb_start_result: Some(ProbeStepResult::Reset),
                usb_tree_result: Some(ProbeStepResult::NotRun),
                usb_reset_result: Some(ProbeStepResult::NotRun),
                final_detected_usb_summary_contains: None,
                clean: Some(false),
            },
        }
    }

    #[must_use]
    pub fn usb_start_timeout() -> Self {
        Self {
            name: "usb-start-timeout".to_owned(),
            description: "usb start produces output but never reaches a prompt or reset."
                .to_owned(),
            expected: UsbProbeValidationExpectation {
                autoboot_result: Some(AutobootResult::Other),
                failure_stage: Some(FailureStage::UsbStart),
                usb_start_result: Some(ProbeStepResult::Timeout),
                usb_tree_result: Some(ProbeStepResult::NotRun),
                usb_reset_result: Some(ProbeStepResult::NotRun),
                final_detected_usb_summary_contains: None,
                clean: Some(false),
            },
        }
    }

    #[must_use]
    pub fn usb_tree_reset() -> Self {
        Self {
            name: "usb-tree-reset".to_owned(),
            description: "usb tree resets while enumerating devices.".to_owned(),
            expected: UsbProbeValidationExpectation {
                autoboot_result: Some(AutobootResult::Other),
                failure_stage: Some(FailureStage::UsbTree1),
                usb_start_result: Some(ProbeStepResult::Ok),
                usb_tree_result: Some(ProbeStepResult::Reset),
                usb_reset_result: Some(ProbeStepResult::NotRun),
                final_detected_usb_summary_contains: None,
                clean: Some(false),
            },
        }
    }

    #[must_use]
    pub fn usb_reset_reset() -> Self {
        Self {
            name: "usb-reset-reset".to_owned(),
            description: "usb reset resets the board before the final tree step.".to_owned(),
            expected: UsbProbeValidationExpectation {
                autoboot_result: Some(AutobootResult::Other),
                failure_stage: Some(FailureStage::UsbReset),
                usb_start_result: Some(ProbeStepResult::Ok),
                usb_tree_result: Some(ProbeStepResult::Ok),
                usb_reset_result: Some(ProbeStepResult::Reset),
                final_detected_usb_summary_contains: Some("Hub".to_owned()),
                clean: Some(false),
            },
        }
    }

    #[must_use]
    pub fn evaluate(&self, trial: &TrialResult) -> UsbProbeValidationOutcome {
        let mut mismatches = Vec::new();
        check_expected(
            "autoboot_result",
            self.expected.autoboot_result,
            trial.autoboot_result,
            &mut mismatches,
        );
        check_expected(
            "failure_stage",
            self.expected.failure_stage,
            trial.failure_stage,
            &mut mismatches,
        );
        check_expected(
            "usb_start_result",
            self.expected.usb_start_result,
            trial.usb_start_result,
            &mut mismatches,
        );
        check_expected(
            "usb_tree_result",
            self.expected.usb_tree_result,
            trial.usb_tree_result,
            &mut mismatches,
        );
        check_expected(
            "usb_reset_result",
            self.expected.usb_reset_result,
            trial.usb_reset_result,
            &mut mismatches,
        );
        check_expected(
            "clean",
            self.expected.clean,
            trial.is_clean(),
            &mut mismatches,
        );

        if let Some(expected_summary) = &self.expected.final_detected_usb_summary_contains {
            let summary = &trial.final_detected_usb_summary;
            if !summary.contains(expected_summary) {
                mismatches.push(format!(
                    "final_detected_usb_summary_contains expected {expected_summary:?}, got {summary:?}"
                ));
            }
        }

        let status = if mismatches.is_empty() {
            UsbProbeValidationStatus::Passed
        } else {
            UsbProbeValidationStatus::Failed
        };

        UsbProbeValidationOutcome {
            scenario_name: self.name.clone(),
            status,
            mismatches,
            trial: trial.clone(),
        }
    }
}

impl UsbProbeValidationReport {
    #[must_use]
    pub fn new(plan: UsbProbeValidationPlan, outcomes: Vec<UsbProbeValidationOutcome>) -> Self {
        Self { plan, outcomes }
    }

    #[must_use]
    pub fn is_passing(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| outcome.status == UsbProbeValidationStatus::Passed)
    }
}

pub fn rockpro64_usb_validation_plan(image: ImageInfo) -> UsbProbeValidationPlan {
    UsbProbeValidationPlan::rockpro64_usb_validation_plan(image)
}

pub fn write_usb_probe_validation_plan_json(
    path: impl AsRef<Path>,
    plan: &UsbProbeValidationPlan,
) -> anyhow::Result<()> {
    write_pretty_json_file(path, plan)
}

pub fn load_usb_probe_validation_plan_json(
    path: impl AsRef<Path>,
) -> anyhow::Result<UsbProbeValidationPlan> {
    read_json_file(path)
}

pub fn write_usb_probe_validation_report_json(
    path: impl AsRef<Path>,
    report: &UsbProbeValidationReport,
) -> anyhow::Result<()> {
    write_pretty_json_file(path, report)
}

pub fn load_usb_probe_validation_report_json(
    path: impl AsRef<Path>,
) -> anyhow::Result<UsbProbeValidationReport> {
    read_json_file(path)
}

fn check_expected<T>(field: &str, expected: Option<T>, actual: T, mismatches: &mut Vec<String>)
where
    T: PartialEq + fmt::Debug,
{
    if let Some(expected) = expected {
        if actual != expected {
            mismatches.push(format!("{field} expected {expected:?}, got {actual:?}"));
        }
    }
}

mod board_wire {
    use super::Board;
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};
    use std::str::FromStr;

    pub fn serialize<S>(board: &Board, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(board.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Board, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Board::from_str(&value).map_err(D::Error::custom)
    }
}
