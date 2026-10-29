use super::SerialRunConfig;
use super::read::{
    ReadOptions, ReadStatus, ResetPolicy, format_expected_patterns, read_until_patterns,
};
use anyhow::{Context, Result};
use bootswain_core::{
    ValidationExecutorKind, ValidationOutcome, ValidationOutcomeStatus, ValidationPlan,
    ValidationScenario, ValidationStep,
};
use bootswain_probe::SerialIo;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub(super) fn run_scenario<T>(
    session: &mut T,
    scenario: &ValidationScenario,
    prompt: &str,
    config: &SerialRunConfig,
    executor: ValidationExecutorKind,
) -> Result<ValidationOutcome>
where
    T: SerialIo,
{
    let scenario_dir = config.out.join(safe_path_segment(&scenario.name));
    fs::create_dir_all(&scenario_dir)
        .with_context(|| format!("failed to create {}", scenario_dir.display()))?;
    let serial_log = scenario_dir.join("serial.log");
    let relative_log = PathBuf::from(safe_path_segment(&scenario.name)).join("serial.log");

    if scenario.steps.is_empty() {
        fs::write(&serial_log, b"")
            .with_context(|| format!("failed to write {}", serial_log.display()))?;
        return Ok(ValidationOutcome {
            scenario: scenario.name.clone(),
            status: ValidationOutcomeStatus::Skipped,
            executor: Some(executor),
            evidence: Vec::new(),
            logs: vec![relative_log],
            failure: Some("scenario has no validation steps".into()),
        });
    }

    let mut log = String::new();
    let mut evidence = Vec::new();
    let result = run_steps(session, scenario, prompt, config, &mut log, &mut evidence);

    // Persist what was captured before reporting, including after an I/O error, so a
    // failed scenario always leaves its serial log behind.
    fs::write(&serial_log, log.as_bytes())
        .with_context(|| format!("failed to write {}", serial_log.display()))?;
    let (status, failure) = match result {
        Ok(StepResult::Passed(_)) => (ValidationOutcomeStatus::Passed, None),
        Ok(StepResult::Failed(failure)) => (ValidationOutcomeStatus::Failed, Some(failure)),
        Ok(StepResult::Interrupted) => (
            ValidationOutcomeStatus::Interrupted,
            Some("validation interrupted by operator".into()),
        ),
        Err(error) => (
            ValidationOutcomeStatus::Failed,
            Some(format!("{}: {error:#}", scenario.name)),
        ),
    };
    Ok(ValidationOutcome {
        scenario: scenario.name.clone(),
        status,
        executor: Some(executor),
        evidence,
        logs: vec![relative_log],
        failure,
    })
}

fn run_steps<T>(
    session: &mut T,
    scenario: &ValidationScenario,
    prompt: &str,
    config: &SerialRunConfig,
    log: &mut String,
    evidence: &mut Vec<String>,
) -> Result<StepResult>
where
    T: SerialIo,
{
    for step in &scenario.steps {
        match run_step(session, scenario, step, prompt, config, log)? {
            StepResult::Passed(mut step_evidence) => evidence.append(&mut step_evidence),
            terminal => return Ok(terminal),
        }
    }
    Ok(StepResult::Passed(Vec::new()))
}

enum StepResult {
    Passed(Vec<String>),
    Failed(String),
    Interrupted,
}

fn run_step<T>(
    session: &mut T,
    scenario: &ValidationScenario,
    step: &ValidationStep,
    prompt: &str,
    config: &SerialRunConfig,
    log: &mut String,
) -> Result<StepResult>
where
    T: SerialIo,
{
    let timeout = effective_step_timeout(step, config);
    progressln(
        config,
        &format!(
            "step {}: waiting up to {}s for {}",
            step.name,
            timeout.as_secs(),
            if step.command.is_some() {
                "U-Boot prompt before command"
            } else {
                "expected serial output"
            }
        ),
    );
    if let Some(command) = &step.command {
        let prompt_read = read_until_patterns(
            session,
            ReadOptions {
                timeout,
                idle_sleep: config.idle_sleep,
                progress_interval: config.progress_interval,
                progress_label: Some(format!("{} prompt", step.name)),
                fail_on_prompt: false,
                repeat_prompt_request: true,
                reset_policy: ResetPolicy::IgnoreBootBanners,
            },
            &config.cancellation,
            log,
            &[prompt.to_owned()],
        )?;
        match prompt_read.status {
            ReadStatus::Matched => {
                let prompt_segment = prompt_read.segment(log);
                if !scenario.is_destructive_spi()
                    && step
                        .expect
                        .iter()
                        .all(|pattern| prompt_segment.contains(pattern))
                {
                    return Ok(StepResult::Passed(
                        step.expect
                            .iter()
                            .map(|pattern| {
                                format!("{} matched serial pattern: {:?}", step.name, pattern)
                            })
                            .collect(),
                    ));
                }
                clear_stale_input_before_command(session, step, log)?;
            }
            ReadStatus::Reset => {
                return Ok(StepResult::Failed(format!(
                    "{}: board reset before command prompt",
                    step.name
                )));
            }
            ReadStatus::Timeout => {
                return Ok(StepResult::Failed(format!(
                    "{}: timed out waiting for U-Boot prompt before command",
                    step.name
                )));
            }
            ReadStatus::Failure(failure) => {
                return Ok(StepResult::Failed(format!(
                    "{}: failed while waiting for U-Boot prompt before command: {failure}",
                    step.name
                )));
            }
            ReadStatus::Interrupted => return Ok(StepResult::Interrupted),
        }

        let _ = write!(log, "\n# bootswain sent: {command}\n");
        progressln(config, &format!("step {}: sending `{command}`", step.name));
        session
            .write_all(format!("{command}\n").as_bytes())
            .with_context(|| format!("failed to write command for step {}", step.name))?;
        session
            .flush()
            .with_context(|| format!("failed to flush command for step {}", step.name))?;
    }

    // A prompt-only step races U-Boot's bootdelay just like a command step, so it also
    // requests the prompt; the boot banner it waits through is expected, not a reset.
    let waits_for_prompt =
        step.command.is_none() && step.expect.iter().any(|pattern| pattern == prompt);
    if waits_for_prompt {
        let _ = write!(
            log,
            "\n# bootswain waiting for U-Boot prompt for {}\n",
            step.name
        );
    }

    let step_read = read_until_patterns(
        session,
        ReadOptions {
            timeout,
            idle_sleep: config.idle_sleep,
            progress_interval: config.progress_interval,
            progress_label: Some(step.name.clone()),
            fail_on_prompt: step.command.is_some(),
            repeat_prompt_request: waits_for_prompt,
            reset_policy: if waits_for_prompt {
                ResetPolicy::IgnoreBootBanners
            } else {
                ResetPolicy::Fail
            },
        },
        &config.cancellation,
        log,
        &step.expect,
    )?;
    match step_read.status {
        ReadStatus::Matched => Ok(StepResult::Passed(
            step.expect
                .iter()
                .map(|pattern| format!("{} matched serial pattern: {:?}", step.name, pattern))
                .collect(),
        )),
        ReadStatus::Reset => Ok(StepResult::Failed(format!(
            "{}: board reset before expected serial patterns appeared",
            step.name
        ))),
        ReadStatus::Timeout => Ok(StepResult::Failed(format!(
            "{}: timed out waiting for expected serial patterns: {}",
            step.name,
            format_expected_patterns(&step.expect)
        ))),
        ReadStatus::Failure(failure) => Ok(StepResult::Failed(format!("{}: {failure}", step.name))),
        ReadStatus::Interrupted => Ok(StepResult::Interrupted),
    }
}

fn clear_stale_input_before_command<T>(
    session: &mut T,
    step: &ValidationStep,
    log: &mut String,
) -> Result<()>
where
    T: SerialIo,
{
    let _ = write!(
        log,
        "\n# bootswain cleared stale serial input before command for {}\n",
        step.name
    );
    session
        .clear()
        .with_context(|| format!("failed to clear serial input before step {}", step.name))
}

fn effective_step_timeout(step: &ValidationStep, config: &SerialRunConfig) -> Duration {
    let plan_timeout = Duration::from_secs(step.timeout_secs);
    match config.max_step_timeout {
        Some(max_timeout) if max_timeout < plan_timeout => max_timeout,
        _ => plan_timeout,
    }
}

pub(super) fn selected_timeout_secs(
    plan: &ValidationPlan,
    selected: &BTreeSet<String>,
    config: &SerialRunConfig,
) -> u64 {
    plan.scenarios
        .iter()
        .filter(|scenario| selected.contains(&scenario.name))
        .flat_map(|scenario| scenario.steps.iter())
        .map(|step| effective_step_timeout(step, config).as_secs())
        .sum()
}

pub(super) fn progressln(config: &SerialRunConfig, message: &str) {
    if config.progress_interval > Duration::ZERO {
        eprintln!("bootswain: {message}");
    }
}

fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}
