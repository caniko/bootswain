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

use scenario::{is_destructive_spi_scenario, progressln, run_scenario, selected_timeout_secs};

#[derive(Debug, Clone)]
pub struct RockPro64SerialRunConfig {
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

pub fn run_rockpro64_serial_validation(
    plan: ValidationPlan,
    config: &RockPro64SerialRunConfig,
) -> Result<ValidationRun> {
    validate_plan_for_rockpro64_serial(&plan)?;
    let _ = selected_scenarios(&plan, &config.selected_scenarios)?;
    let mut serial = RealSerial::open(&config.port, config.baud)?;
    run_rockpro64_serial_validation_with_session(plan, config, &mut serial)
}

pub fn run_rockpro64_serial_validation_with_session<T>(
    plan: ValidationPlan,
    config: &RockPro64SerialRunConfig,
    session: &mut T,
) -> Result<ValidationRun>
where
    T: SerialIo,
{
    validate_plan_for_rockpro64_serial(&plan)?;
    fs::create_dir_all(&config.out)
        .with_context(|| format!("failed to create {}", config.out.display()))?;

    let selected = selected_scenarios(&plan, &config.selected_scenarios)?;
    let _ = session.clear();

    progressln(
        config,
        &format!(
            "ROCKPro64 serial validation: {} selected scenario(s), max planned wait {}s",
            selected.len(),
            selected_timeout_secs(&plan, &selected, config)
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
                executor: Some(ValidationExecutorKind::RockPro64Serial),
                evidence: Vec::new(),
                logs: Vec::new(),
                failure: Some("scenario not selected for this hardware run".into()),
            });
            continue;
        }

        if interrupted {
            outcomes.push(interrupted_outcome(scenario));
            continue;
        }

        if let Some(failed_scenario) = &stopped_after {
            outcomes.push(stopped_after_failure_outcome(scenario, failed_scenario));
            continue;
        }

        if is_destructive_spi_scenario(scenario) && !config.allow_destructive_spi {
            outcomes.push(ValidationOutcome {
                scenario: scenario.name.clone(),
                status: ValidationOutcomeStatus::Skipped,
                executor: Some(ValidationExecutorKind::RockPro64Serial),
                evidence: Vec::new(),
                logs: Vec::new(),
                failure: Some("destructive SPI scenario requires --allow-destructive-spi".into()),
            });
            continue;
        }

        progressln(config, &format!("scenario {}: starting", scenario.name));
        let outcome = run_scenario(session, scenario, config)?;
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
pub fn rockpro64_serial_selected_outcomes_passed(run: &ValidationRun) -> bool {
    run.outcomes.iter().all(|outcome| {
        outcome.status == ValidationOutcomeStatus::Passed
            || outcome.status == ValidationOutcomeStatus::NotRun
    })
}

fn validate_plan_for_rockpro64_serial(plan: &ValidationPlan) -> Result<()> {
    if plan.board != Board::RockPro64 {
        bail!("rockpro64-serial validates ROCKPro64 plans only");
    }
    if plan.executor != ValidationExecutorKind::RockPro64Serial {
        bail!(
            "rockpro64-serial requires executor rockpro64-serial, plan has {}",
            plan.executor
        );
    }
    Ok(())
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

fn interrupted_outcome(scenario: &ValidationScenario) -> ValidationOutcome {
    ValidationOutcome {
        scenario: scenario.name.clone(),
        status: ValidationOutcomeStatus::Interrupted,
        executor: Some(ValidationExecutorKind::RockPro64Serial),
        evidence: Vec::new(),
        logs: Vec::new(),
        failure: Some("validation interrupted by operator".into()),
    }
}

fn stopped_after_failure_outcome(
    scenario: &ValidationScenario,
    failed_scenario: &str,
) -> ValidationOutcome {
    ValidationOutcome {
        scenario: scenario.name.clone(),
        status: ValidationOutcomeStatus::NotRun,
        executor: Some(ValidationExecutorKind::RockPro64Serial),
        evidence: Vec::new(),
        logs: Vec::new(),
        failure: Some(format!(
            "run stopped after failed scenario: {failed_scenario}"
        )),
    }
}

#[cfg(test)]
mod tests;
