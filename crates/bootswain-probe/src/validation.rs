use bootswain_core::{
    AutobootResult, Board, FailureStage, ImageInfo, ProbeStepResult, TrialResult,
};
use std::fmt;
use std::fs;
use std::iter::FromIterator;
use std::path::Path;

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationPlan {
    pub name: String,
    pub board: Board,
    pub image: ImageInfo,
    pub scenarios: Vec<ValidationScenario>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationScenario {
    pub name: String,
    pub description: String,
    pub expected: ValidationExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationExpectation {
    pub autoboot_result: Option<AutobootResult>,
    pub failure_stage: Option<FailureStage>,
    pub usb_start_result: Option<ProbeStepResult>,
    pub usb_tree_result: Option<ProbeStepResult>,
    pub usb_reset_result: Option<ProbeStepResult>,
    pub final_detected_usb_summary_contains: Option<String>,
    pub clean: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    Passed,
    Failed,
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passed => f.write_str("passed"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationOutcome {
    pub scenario_name: String,
    pub status: ValidationStatus,
    pub mismatches: Vec<String>,
    pub trial: TrialResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub plan: ValidationPlan,
    pub outcomes: Vec<ValidationOutcome>,
}

impl ValidationPlan {
    #[must_use]
    pub fn rockpro64_usb_validation_plan(image: ImageInfo) -> Self {
        Self {
            name: "rockpro64-usb-validation".to_owned(),
            board: Board::RockPro64,
            image,
            scenarios: vec![
                ValidationScenario::clean_usb_sequence(),
                ValidationScenario::autoboot_reset(),
                ValidationScenario::usb_start_reset(),
                ValidationScenario::usb_start_timeout(),
                ValidationScenario::usb_tree_reset(),
                ValidationScenario::usb_reset_reset(),
            ],
        }
    }

    #[must_use]
    pub fn scenario(&self, name: &str) -> Option<&ValidationScenario> {
        self.scenarios.iter().find(|scenario| scenario.name == name)
    }
}

impl ValidationScenario {
    #[must_use]
    pub fn clean_usb_sequence() -> Self {
        Self {
            name: "clean-usb-sequence".to_owned(),
            description: "U-Boot prompt is reached and the full USB probe sequence succeeds."
                .to_owned(),
            expected: ValidationExpectation {
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
            expected: ValidationExpectation {
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
            expected: ValidationExpectation {
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
            expected: ValidationExpectation {
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
            expected: ValidationExpectation {
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
            expected: ValidationExpectation {
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
    pub fn evaluate(&self, trial: &TrialResult) -> ValidationOutcome {
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
            ValidationStatus::Passed
        } else {
            ValidationStatus::Failed
        };

        ValidationOutcome {
            scenario_name: self.name.clone(),
            status,
            mismatches,
            trial: trial.clone(),
        }
    }
}

impl ValidationReport {
    #[must_use]
    pub fn new(plan: ValidationPlan, outcomes: Vec<ValidationOutcome>) -> Self {
        Self { plan, outcomes }
    }

    #[must_use]
    pub fn is_passing(&self) -> bool {
        self.outcomes
            .iter()
            .all(|outcome| outcome.status == ValidationStatus::Passed)
    }
}

pub fn rockpro64_usb_validation_plan(image: ImageInfo) -> ValidationPlan {
    ValidationPlan::rockpro64_usb_validation_plan(image)
}

pub fn write_validation_plan_json(
    path: impl AsRef<Path>,
    plan: &ValidationPlan,
) -> anyhow::Result<()> {
    write_json(path.as_ref(), &validation_plan_to_value(plan))
}

pub fn load_validation_plan_json(path: impl AsRef<Path>) -> anyhow::Result<ValidationPlan> {
    let value = read_json(path.as_ref())?;
    validation_plan_from_value(&value)
}

pub fn write_validation_report_json(
    path: impl AsRef<Path>,
    report: &ValidationReport,
) -> anyhow::Result<()> {
    write_json(path.as_ref(), &validation_report_to_value(report))
}

pub fn load_validation_report_json(path: impl AsRef<Path>) -> anyhow::Result<ValidationReport> {
    let value = read_json(path.as_ref())?;
    validation_report_from_value(&value)
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

fn write_json(path: &Path, value: &Value) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json)?;
    Ok(())
}

fn read_json(path: &Path) -> anyhow::Result<Value> {
    let json = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

fn validation_plan_to_value(plan: &ValidationPlan) -> Value {
    Value::Object(Map::from_iter([
        ("name".to_owned(), Value::String(plan.name.clone())),
        ("board".to_owned(), Value::String(plan.board.to_string())),
        (
            "image".to_owned(),
            serde_json::to_value(&plan.image).expect("image serializes"),
        ),
        (
            "scenarios".to_owned(),
            Value::Array(
                plan.scenarios
                    .iter()
                    .map(validation_scenario_to_value)
                    .collect(),
            ),
        ),
    ]))
}

fn validation_plan_from_value(value: &Value) -> anyhow::Result<ValidationPlan> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("validation plan must be a JSON object"))?;

    Ok(ValidationPlan {
        name: string_field(object, "name")?.to_owned(),
        board: board_from_string(string_field(object, "board")?)?,
        image: serde_json::from_value(
            object
                .get("image")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("validation plan is missing image"))?,
        )?,
        scenarios: array_field(object, "scenarios")?
            .iter()
            .map(validation_scenario_from_value)
            .collect::<anyhow::Result<Vec<_>>>()?,
    })
}

fn validation_scenario_to_value(scenario: &ValidationScenario) -> Value {
    Value::Object(Map::from_iter([
        ("name".to_owned(), Value::String(scenario.name.clone())),
        (
            "description".to_owned(),
            Value::String(scenario.description.clone()),
        ),
        (
            "expected".to_owned(),
            validation_expectation_to_value(&scenario.expected),
        ),
    ]))
}

fn validation_scenario_from_value(value: &Value) -> anyhow::Result<ValidationScenario> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("validation scenario must be a JSON object"))?;

    Ok(ValidationScenario {
        name: string_field(object, "name")?.to_owned(),
        description: string_field(object, "description")?.to_owned(),
        expected: validation_expectation_from_value(
            object
                .get("expected")
                .ok_or_else(|| anyhow::anyhow!("validation scenario is missing expected"))?,
        )?,
    })
}

fn validation_expectation_to_value(expected: &ValidationExpectation) -> Value {
    let mut object = Map::new();
    if let Some(value) = expected.autoboot_result {
        object.insert(
            "autoboot_result".to_owned(),
            Value::String(autoboot_to_string(value).to_owned()),
        );
    }
    if let Some(value) = expected.failure_stage {
        object.insert(
            "failure_stage".to_owned(),
            Value::String(failure_stage_to_string(value).to_owned()),
        );
    }
    if let Some(value) = expected.usb_start_result {
        object.insert(
            "usb_start_result".to_owned(),
            Value::String(step_result_to_string(value).to_owned()),
        );
    }
    if let Some(value) = expected.usb_tree_result {
        object.insert(
            "usb_tree_result".to_owned(),
            Value::String(step_result_to_string(value).to_owned()),
        );
    }
    if let Some(value) = expected.usb_reset_result {
        object.insert(
            "usb_reset_result".to_owned(),
            Value::String(step_result_to_string(value).to_owned()),
        );
    }
    if let Some(value) = &expected.final_detected_usb_summary_contains {
        object.insert(
            "final_detected_usb_summary_contains".to_owned(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = expected.clean {
        object.insert("clean".to_owned(), Value::Bool(value));
    }
    Value::Object(object)
}

fn validation_expectation_from_value(value: &Value) -> anyhow::Result<ValidationExpectation> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("validation expectation must be a JSON object"))?;

    Ok(ValidationExpectation {
        autoboot_result: optional_enum_field(object, "autoboot_result", autoboot_from_string)?,
        failure_stage: optional_enum_field(object, "failure_stage", failure_stage_from_string)?,
        usb_start_result: optional_enum_field(object, "usb_start_result", step_result_from_string)?,
        usb_tree_result: optional_enum_field(object, "usb_tree_result", step_result_from_string)?,
        usb_reset_result: optional_enum_field(object, "usb_reset_result", step_result_from_string)?,
        final_detected_usb_summary_contains: optional_string_field(
            object,
            "final_detected_usb_summary_contains",
        )?
        .map(|value| value.to_owned()),
        clean: optional_bool_field(object, "clean")?,
    })
}

fn validation_report_to_value(report: &ValidationReport) -> Value {
    Value::Object(Map::from_iter([
        ("plan".to_owned(), validation_plan_to_value(&report.plan)),
        (
            "outcomes".to_owned(),
            Value::Array(
                report
                    .outcomes
                    .iter()
                    .map(validation_outcome_to_value)
                    .collect(),
            ),
        ),
    ]))
}

fn validation_report_from_value(value: &Value) -> anyhow::Result<ValidationReport> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("validation report must be a JSON object"))?;

    Ok(ValidationReport {
        plan: validation_plan_from_value(
            object
                .get("plan")
                .ok_or_else(|| anyhow::anyhow!("validation report is missing plan"))?,
        )?,
        outcomes: array_field(object, "outcomes")?
            .iter()
            .map(validation_outcome_from_value)
            .collect::<anyhow::Result<Vec<_>>>()?,
    })
}

fn validation_outcome_to_value(outcome: &ValidationOutcome) -> Value {
    Value::Object(Map::from_iter([
        (
            "scenario_name".to_owned(),
            Value::String(outcome.scenario_name.clone()),
        ),
        (
            "status".to_owned(),
            Value::String(outcome.status.to_string()),
        ),
        (
            "mismatches".to_owned(),
            Value::Array(
                outcome
                    .mismatches
                    .iter()
                    .map(|mismatch| Value::String(mismatch.clone()))
                    .collect(),
            ),
        ),
        (
            "trial".to_owned(),
            serde_json::to_value(&outcome.trial).expect("trial serializes"),
        ),
    ]))
}

fn validation_outcome_from_value(value: &Value) -> anyhow::Result<ValidationOutcome> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("validation outcome must be a JSON object"))?;

    Ok(ValidationOutcome {
        scenario_name: string_field(object, "scenario_name")?.to_owned(),
        status: validation_status_from_string(string_field(object, "status")?)?,
        mismatches: array_field(object, "mismatches")?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("mismatches entries must be strings"))
                    .map(|entry| entry.to_owned())
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
        trial: serde_json::from_value(
            object
                .get("trial")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("validation outcome is missing trial"))?,
        )?,
    })
}

fn string_field<'a>(object: &'a Map<String, Value>, key: &str) -> anyhow::Result<&'a str> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing or invalid string field {key}"))
}

fn optional_string_field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> anyhow::Result<Option<&'a str>> {
    match object.get(key) {
        Some(value) => value
            .as_str()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("invalid string field {key}")),
        None => Ok(None),
    }
}

fn optional_bool_field(object: &Map<String, Value>, key: &str) -> anyhow::Result<Option<bool>> {
    match object.get(key) {
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("invalid bool field {key}")),
        None => Ok(None),
    }
}

fn array_field<'a>(object: &'a Map<String, Value>, key: &str) -> anyhow::Result<&'a [Value]> {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| anyhow::anyhow!("missing or invalid array field {key}"))
}

fn optional_enum_field<T>(
    object: &Map<String, Value>,
    key: &str,
    parse: fn(&str) -> anyhow::Result<T>,
) -> anyhow::Result<Option<T>> {
    match object.get(key) {
        Some(value) => value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("invalid string field {key}"))
            .and_then(parse)
            .map(Some),
        None => Ok(None),
    }
}

fn validation_status_from_string(value: &str) -> anyhow::Result<ValidationStatus> {
    match value {
        "passed" => Ok(ValidationStatus::Passed),
        "failed" => Ok(ValidationStatus::Failed),
        other => Err(anyhow::anyhow!("unknown validation status {other}")),
    }
}

fn autoboot_from_string(value: &str) -> anyhow::Result<AutobootResult> {
    match value {
        "abort" => Ok(AutobootResult::Abort),
        "warn-enumerate" => Ok(AutobootResult::WarnEnumerate),
        "no-device" => Ok(AutobootResult::NoDevice),
        "other" => Ok(AutobootResult::Other),
        other => Err(anyhow::anyhow!("unknown autoboot result {other}")),
    }
}

fn failure_stage_from_string(value: &str) -> anyhow::Result<FailureStage> {
    match value {
        "none" => Ok(FailureStage::None),
        "autoboot" => Ok(FailureStage::Autoboot),
        "usb-start" => Ok(FailureStage::UsbStart),
        "usb-tree-1" => Ok(FailureStage::UsbTree1),
        "usb-reset" => Ok(FailureStage::UsbReset),
        "usb-tree-2" => Ok(FailureStage::UsbTree2),
        other => Err(anyhow::anyhow!("unknown failure stage {other}")),
    }
}

fn step_result_from_string(value: &str) -> anyhow::Result<ProbeStepResult> {
    match value {
        "ok" => Ok(ProbeStepResult::Ok),
        "reset" => Ok(ProbeStepResult::Reset),
        "timeout" => Ok(ProbeStepResult::Timeout),
        "not-run" => Ok(ProbeStepResult::NotRun),
        other => Err(anyhow::anyhow!("unknown probe step result {other}")),
    }
}

fn board_from_string(value: &str) -> anyhow::Result<Board> {
    match value {
        "rockpro64" => Ok(Board::RockPro64),
        other => Err(anyhow::anyhow!("unknown board {other}")),
    }
}

fn autoboot_to_string(value: AutobootResult) -> &'static str {
    match value {
        AutobootResult::Abort => "abort",
        AutobootResult::WarnEnumerate => "warn-enumerate",
        AutobootResult::NoDevice => "no-device",
        AutobootResult::Other => "other",
    }
}

fn failure_stage_to_string(value: FailureStage) -> &'static str {
    match value {
        FailureStage::None => "none",
        FailureStage::Autoboot => "autoboot",
        FailureStage::UsbStart => "usb-start",
        FailureStage::UsbTree1 => "usb-tree-1",
        FailureStage::UsbReset => "usb-reset",
        FailureStage::UsbTree2 => "usb-tree-2",
    }
}

fn step_result_to_string(value: ProbeStepResult) -> &'static str {
    match value {
        ProbeStepResult::Ok => "ok",
        ProbeStepResult::Reset => "reset",
        ProbeStepResult::Timeout => "timeout",
        ProbeStepResult::NotRun => "not-run",
    }
}
