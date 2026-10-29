use anyhow::{Context, Result, bail};
use bootswain_core::{
    Board, ValidationExecutorKind, ValidationOutcome, ValidationOutcomeStatus, ValidationPlan,
    ValidationRun, ValidationScenario, write_pretty_json_file,
};
use bootswain_probe::{RealSerial, SerialIo};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

mod read;
mod scenario;

use scenario::{progressln, run_scenario, selected_timeout_secs};

#[derive(Debug, Clone)]
pub struct SerialRunConfig {
    pub port: String,
    pub baud: u32,
    pub out: PathBuf,
    pub selected_scenarios: Vec<String>,
    pub allow_destructive_spi: bool,
    pub idle_sleep: Duration,
    pub progress_interval: Duration,
    pub max_step_timeout: Option<Duration>,
    pub cancellation: Arc<AtomicBool>,
}

pub fn run_serial_validation(
    plan: ValidationPlan,
    config: &SerialRunConfig,
) -> Result<ValidationRun> {
    // Validate before touching the port.
    let selected = prepare(&plan, config)?;
    let mut serial = RealSerial::open(&config.port, config.baud)?;
    execute(plan, config, &mut serial, &selected)
}

pub fn run_serial_validation_with_session<T>(
    plan: ValidationPlan,
    config: &SerialRunConfig,
    session: &mut T,
) -> Result<ValidationRun>
where
    T: SerialIo,
{
    let selected = prepare(&plan, config)?;
    execute(plan, config, session, &selected)
}

fn prepare(plan: &ValidationPlan, config: &SerialRunConfig) -> Result<BTreeSet<String>> {
    validate_plan_for_serial(plan)?;
    selected_scenarios(plan, &config.selected_scenarios)
}

fn execute<T>(
    plan: ValidationPlan,
    config: &SerialRunConfig,
    session: &mut T,
    selected: &BTreeSet<String>,
) -> Result<ValidationRun>
where
    T: SerialIo,
{
    fs::create_dir_all(&config.out)
        .with_context(|| format!("failed to create {}", config.out.display()))?;
    let executor = plan.executor;
    let _ = session.clear();

    progressln(
        config,
        &format!(
            "{} serial validation: {} selected scenario(s), max planned wait {}s",
            plan.board,
            selected.len(),
            selected_timeout_secs(&plan, selected, config)
        ),
    );

    let mut outcomes = Vec::with_capacity(plan.scenarios.len());
    let mut stopped_after: Option<String> = None;
    let mut interrupted = false;
    for scenario in &plan.scenarios {
        if !selected.contains(&scenario.name) {
            outcomes.push(ValidationOutcome {
                scenario: scenario.name.clone(),
                status: ValidationOutcomeStatus::NotRun,
                executor: Some(executor),
                evidence: Vec::new(),
                logs: Vec::new(),
                failure: Some("scenario not selected for this hardware run".into()),
            });
            continue;
        }

        if interrupted {
            outcomes.push(interrupted_outcome(scenario, executor));
            continue;
        }

        if let Some(failed_scenario) = &stopped_after {
            outcomes.push(stopped_after_failure_outcome(
                scenario,
                failed_scenario,
                executor,
            ));
            continue;
        }

        if scenario.is_destructive_spi() && !config.allow_destructive_spi {
            outcomes.push(ValidationOutcome {
                scenario: scenario.name.clone(),
                status: ValidationOutcomeStatus::Skipped,
                executor: Some(executor),
                evidence: Vec::new(),
                logs: Vec::new(),
                failure: Some("destructive SPI scenario requires --allow-destructive-spi".into()),
            });
            continue;
        }

        progressln(config, &format!("scenario {}: starting", scenario.name));
        let outcome = run_scenario(session, scenario, config, executor)?;
        match outcome.status {
            ValidationOutcomeStatus::Failed => stopped_after = Some(outcome.scenario.clone()),
            ValidationOutcomeStatus::Interrupted => interrupted = true,
            _ => {}
        }
        outcomes.push(outcome);
    }

    let run = ValidationRun { plan, outcomes };
    write_pretty_json_file(config.out.join("validation-run.json"), &run)?;
    Ok(run)
}

#[must_use]
pub fn serial_selected_outcomes_passed(run: &ValidationRun) -> bool {
    run.outcomes.iter().all(|outcome| {
        outcome.status == ValidationOutcomeStatus::Passed
            || outcome.status == ValidationOutcomeStatus::NotRun
    })
}

fn validate_plan_for_serial(plan: &ValidationPlan) -> Result<()> {
    match plan.executor {
        ValidationExecutorKind::UBootSerial => Ok(()),
        ValidationExecutorKind::RockPro64Serial if plan.board == Board::RockPro64 => Ok(()),
        ValidationExecutorKind::RockPro64Serial => bail!(
            "executor rockpro64-serial is only valid for the rockpro64 board, plan is for {}; use u-boot-serial",
            plan.board
        ),
        other => bail!(
            "serial validation requires executor u-boot-serial (or rockpro64-serial for rockpro64), plan has {other}"
        ),
    }
}

fn selected_scenarios(plan: &ValidationPlan, requested: &[String]) -> Result<BTreeSet<String>> {
    let all = plan
        .scenarios
        .iter()
        .map(|scenario| scenario.name.clone())
        .collect::<BTreeSet<_>>();
    if requested.is_empty() {
        return Ok(all);
    }

    let requested = requested.iter().cloned().collect::<BTreeSet<_>>();
    let missing = requested.difference(&all).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "validation plan does not contain scenario(s): {}",
            missing.join(", ")
        );
    }

    Ok(requested)
}

fn interrupted_outcome(
    scenario: &ValidationScenario,
    executor: ValidationExecutorKind,
) -> ValidationOutcome {
    ValidationOutcome {
        scenario: scenario.name.clone(),
        status: ValidationOutcomeStatus::Interrupted,
        executor: Some(executor),
        evidence: Vec::new(),
        logs: Vec::new(),
        failure: Some("validation interrupted by operator".into()),
    }
}

fn stopped_after_failure_outcome(
    scenario: &ValidationScenario,
    failed_scenario: &str,
    executor: ValidationExecutorKind,
) -> ValidationOutcome {
    ValidationOutcome {
        scenario: scenario.name.clone(),
        status: ValidationOutcomeStatus::NotRun,
        executor: Some(executor),
        evidence: Vec::new(),
        logs: Vec::new(),
        failure: Some(format!(
            "run stopped after failed scenario: {failed_scenario}"
        )),
    }
}

#[cfg(test)]
mod tests;
