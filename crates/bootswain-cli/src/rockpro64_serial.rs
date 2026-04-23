use anyhow::{Context, Result, bail};
use bootswain_core::{
    Board, ValidationExecutorKind, ValidationOutcome, ValidationOutcomeStatus, ValidationPlan,
    ValidationRun, ValidationScenario, ValidationScenarioKind, ValidationStep,
};
use bootswain_probe::{RealSerial, SerialIo, segment_has_reset};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RockPro64SerialRunConfig {
    pub port: String,
    pub baud: u32,
    pub out: PathBuf,
    pub selected_scenarios: Vec<String>,
    pub allow_destructive_spi: bool,
    pub idle_sleep: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadStatus {
    Matched,
    Reset,
    Timeout,
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

    let mut outcomes = Vec::with_capacity(plan.scenarios.len());
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

        outcomes.push(run_scenario(session, scenario, config)?);
    }

    let run = ValidationRun { plan, outcomes };
    write_json(&config.out.join("validation-run.json"), &run)?;
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

fn run_scenario<T>(
    session: &mut T,
    scenario: &ValidationScenario,
    config: &RockPro64SerialRunConfig,
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
            executor: Some(ValidationExecutorKind::RockPro64Serial),
            evidence: Vec::new(),
            logs: vec![relative_log],
            failure: Some("scenario has no validation steps".into()),
        });
    }

    let mut log = String::new();
    let mut evidence = Vec::new();
    for step in &scenario.steps {
        match run_step(session, step, config, &mut log)? {
            StepResult::Passed(mut step_evidence) => evidence.append(&mut step_evidence),
            StepResult::Failed(failure) => {
                fs::write(&serial_log, log.as_bytes())
                    .with_context(|| format!("failed to write {}", serial_log.display()))?;
                return Ok(ValidationOutcome {
                    scenario: scenario.name.clone(),
                    status: ValidationOutcomeStatus::Failed,
                    executor: Some(ValidationExecutorKind::RockPro64Serial),
                    evidence,
                    logs: vec![relative_log],
                    failure: Some(failure),
                });
            }
        }
    }

    fs::write(&serial_log, log.as_bytes())
        .with_context(|| format!("failed to write {}", serial_log.display()))?;
    Ok(ValidationOutcome {
        scenario: scenario.name.clone(),
        status: ValidationOutcomeStatus::Passed,
        executor: Some(ValidationExecutorKind::RockPro64Serial),
        evidence,
        logs: vec![relative_log],
        failure: None,
    })
}

enum StepResult {
    Passed(Vec<String>),
    Failed(String),
}

fn run_step<T>(
    session: &mut T,
    step: &ValidationStep,
    config: &RockPro64SerialRunConfig,
    log: &mut String,
) -> Result<StepResult>
where
    T: SerialIo,
{
    let timeout = Duration::from_secs(step.timeout_secs);
    if let Some(command) = &step.command {
        let mut prompt_segment = String::new();
        match read_until_patterns(
            session,
            timeout,
            config.idle_sleep,
            log,
            &mut prompt_segment,
            &["=> ".to_owned()],
        )? {
            ReadStatus::Matched => {}
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
        }

        log.push_str(&format!("\n# bootswain sent: {command}\n"));
        session
            .write_all(format!("{command}\n").as_bytes())
            .with_context(|| format!("failed to write command for step {}", step.name))?;
        session
            .flush()
            .with_context(|| format!("failed to flush command for step {}", step.name))?;
    }

    let mut step_segment = String::new();
    match read_until_patterns(
        session,
        timeout,
        config.idle_sleep,
        log,
        &mut step_segment,
        &step.expect,
    )? {
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
            step.expect
                .iter()
                .map(|pattern| format!("{pattern:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn read_until_patterns<T>(
    session: &mut T,
    timeout: Duration,
    idle_sleep: Duration,
    full_log: &mut String,
    segment_log: &mut String,
    patterns: &[String],
) -> Result<ReadStatus>
where
    T: SerialIo,
{
    if patterns.is_empty() {
        return Ok(ReadStatus::Matched);
    }

    let start = Instant::now();
    let mut buffer = [0_u8; 1024];
    while start.elapsed() < timeout {
        let read = session.read_chunk(&mut buffer)?;
        if read == 0 {
            if !idle_sleep.is_zero() {
                thread::sleep(idle_sleep);
            }
            continue;
        }

        let chunk = String::from_utf8_lossy(&buffer[..read]);
        full_log.push_str(&chunk);
        segment_log.push_str(&chunk);

        if segment_has_reset(segment_log) {
            return Ok(ReadStatus::Reset);
        }
        if patterns.iter().all(|pattern| segment_log.contains(pattern)) {
            return Ok(ReadStatus::Matched);
        }
    }

    Ok(ReadStatus::Timeout)
}

fn is_destructive_spi_scenario(scenario: &ValidationScenario) -> bool {
    matches!(
        scenario.kind,
        ValidationScenarioKind::SpiInstall | ValidationScenarioKind::SpiErase
    )
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

fn write_json<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    fs::write(
        path,
        serde_json::to_string_pretty(value).context("failed to encode JSON")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{
        RockPro64SerialRunConfig, rockpro64_serial_selected_outcomes_passed,
        run_rockpro64_serial_validation_with_session,
    };
    use bootswain_core::{
        Board, BootProtocol, BootTarget, ValidationExecutorKind, ValidationOutcomeStatus,
        ValidationPlan, ValidationScenario, ValidationScenarioKind, ValidationStep,
    };
    use bootswain_probe::SerialIo;
    use std::collections::VecDeque;
    use std::io;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::tempdir;

    #[derive(Debug, Default)]
    struct FakeSerial {
        chunks: VecDeque<Vec<u8>>,
        writes: Vec<String>,
    }

    impl FakeSerial {
        fn with_chunks(chunks: &[&str]) -> Self {
            Self {
                chunks: chunks
                    .iter()
                    .map(|chunk| chunk.as_bytes().to_vec())
                    .collect(),
                writes: Vec::new(),
            }
        }
    }

    impl SerialIo for FakeSerial {
        fn clear(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let Some(chunk) = self.chunks.pop_front() else {
                return Ok(0);
            };
            let read = chunk.len().min(buffer.len());
            buffer[..read].copy_from_slice(&chunk[..read]);
            Ok(read)
        }

        fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
            self.writes
                .push(String::from_utf8_lossy(buffer).to_string());
            Ok(())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn prompt_scenario_passes_and_writes_log() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(
            tmp.path().join("out"),
            vec!["recovery-console".into()],
            false,
        );
        let mut serial = FakeSerial::with_chunks(&["U-Boot 2026.04\n=> "]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "recovery-console")
            .expect("recovery outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
        assert!(rockpro64_serial_selected_outcomes_passed(&run));
        assert!(config.out.join("recovery-console/serial.log").exists());
    }

    #[test]
    fn command_step_waits_for_prompt_before_write() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
        let mut serial = FakeSerial::with_chunks(&["=> ", "Scanning for bootflows\n=> "]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "bootflow")
            .expect("bootflow outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
        assert_eq!(serial.writes, vec!["bootflow scan\n"]);
    }

    #[test]
    fn timeout_marks_selected_scenario_failed() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(
            tmp.path().join("out"),
            vec!["recovery-console".into()],
            false,
        );
        let mut serial = FakeSerial::default();
        let mut plan = sample_plan();
        plan.scenarios[0].steps[0].timeout_secs = 0;

        let run = run_rockpro64_serial_validation_with_session(plan, &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "recovery-console")
            .expect("recovery outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
        assert!(
            outcome
                .failure
                .as_deref()
                .expect("failure")
                .contains("timed out")
        );
        assert!(!rockpro64_serial_selected_outcomes_passed(&run));
    }

    #[test]
    fn reset_marks_selected_scenario_failed() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(
            tmp.path().join("out"),
            vec!["recovery-console".into()],
            false,
        );
        let mut serial = FakeSerial::with_chunks(&["Resetting CPU ..."]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "recovery-console")
            .expect("recovery outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
        assert!(
            outcome
                .failure
                .as_deref()
                .expect("failure")
                .contains("reset")
        );
    }

    #[test]
    fn destructive_spi_scenarios_are_skipped_without_flag() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], false);
        let mut serial = FakeSerial::with_chunks(&["=> "]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Skipped);
        assert!(serial.writes.is_empty());
        assert!(!rockpro64_serial_selected_outcomes_passed(&run));
    }

    #[test]
    fn destructive_spi_scenarios_run_with_flag() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
        let mut serial = FakeSerial::with_chunks(&["=> ", "SPI flash complete\n"]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
        assert_eq!(serial.writes, vec!["run bootswain_flash_spi\n"]);
    }

    #[test]
    fn missing_scenario_filter_is_rejected() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["missing".into()], false);
        let mut serial = FakeSerial::default();

        let error =
            run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
                .expect_err("missing scenario should fail");

        assert!(error.to_string().contains("does not contain scenario"));
    }

    fn sample_config(
        out: PathBuf,
        selected_scenarios: Vec<String>,
        allow_destructive_spi: bool,
    ) -> RockPro64SerialRunConfig {
        RockPro64SerialRunConfig {
            port: "/dev/ttyUSB0".into(),
            baud: 115_200,
            out,
            selected_scenarios,
            allow_destructive_spi,
            idle_sleep: Duration::ZERO,
        }
    }

    fn sample_plan() -> ValidationPlan {
        ValidationPlan {
            schema_version: 1,
            board: Board::RockPro64,
            release: "test".into(),
            executor: ValidationExecutorKind::RockPro64Serial,
            qemu: None,
            scenarios: vec![
                ValidationScenario {
                    name: "recovery-console".into(),
                    kind: ValidationScenarioKind::Prompt,
                    target: Some(BootTarget::Spi),
                    protocol: Some(BootProtocol::UBootShell),
                    artifact: None,
                    steps: vec![ValidationStep {
                        name: "prompt".into(),
                        command: None,
                        expect: vec!["=> ".into()],
                        timeout_secs: 1,
                    }],
                },
                ValidationScenario {
                    name: "bootflow".into(),
                    kind: ValidationScenarioKind::BootflowScan,
                    target: Some(BootTarget::Sd),
                    protocol: None,
                    artifact: None,
                    steps: vec![ValidationStep {
                        name: "scan".into(),
                        command: Some("bootflow scan".into()),
                        expect: vec!["Scanning for bootflows".into()],
                        timeout_secs: 1,
                    }],
                },
                ValidationScenario {
                    name: "spi-install".into(),
                    kind: ValidationScenarioKind::SpiInstall,
                    target: Some(BootTarget::Spi),
                    protocol: Some(BootProtocol::UBootShell),
                    artifact: None,
                    steps: vec![ValidationStep {
                        name: "flash spi".into(),
                        command: Some("run bootswain_flash_spi".into()),
                        expect: vec!["SPI flash complete".into()],
                        timeout_secs: 1,
                    }],
                },
            ],
        }
    }
}
