use anyhow::{Context, Result, bail};
use bootswain_core::{
    Board, ValidationExecutorKind, ValidationOutcome, ValidationOutcomeStatus, ValidationPlan,
    ValidationRun, ValidationScenario, ValidationScenarioKind, ValidationStep,
    write_pretty_json_file,
};
use bootswain_probe::{RealSerial, SerialIo};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
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
    pub progress_interval: Duration,
    pub max_step_timeout: Option<Duration>,
    pub cancellation: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReadStatus {
    Matched,
    Reset,
    Failure(String),
    Interrupted,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResetPolicy {
    Fail,
    IgnoreBootBanners,
}

#[derive(Debug, Clone)]
struct ReadOptions {
    timeout: Duration,
    idle_sleep: Duration,
    progress_interval: Duration,
    progress_label: Option<String>,
    fail_on_prompt: bool,
    repeat_prompt_request: bool,
    reset_policy: ResetPolicy,
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
        match run_step(session, scenario, step, config, &mut log)? {
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
            StepResult::Interrupted => {
                fs::write(&serial_log, log.as_bytes())
                    .with_context(|| format!("failed to write {}", serial_log.display()))?;
                return Ok(ValidationOutcome {
                    scenario: scenario.name.clone(),
                    status: ValidationOutcomeStatus::Interrupted,
                    executor: Some(ValidationExecutorKind::RockPro64Serial),
                    evidence,
                    logs: vec![relative_log],
                    failure: Some("validation interrupted by operator".into()),
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
    Interrupted,
}

fn run_step<T>(
    session: &mut T,
    scenario: &ValidationScenario,
    step: &ValidationStep,
    config: &RockPro64SerialRunConfig,
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
        let mut prompt_segment = String::new();
        match read_until_patterns(
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
            &mut prompt_segment,
            &["=> ".to_owned()],
        )? {
            ReadStatus::Matched => {
                if command_step_allows_precommand_match(scenario)
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

        log.push_str(&format!("\n# bootswain sent: {command}\n"));
        progressln(config, &format!("step {}: sending `{command}`", step.name));
        session
            .write_all(format!("{command}\n").as_bytes())
            .with_context(|| format!("failed to write command for step {}", step.name))?;
        session
            .flush()
            .with_context(|| format!("failed to flush command for step {}", step.name))?;
    }

    if step.command.is_none() && step.expect.iter().any(|pattern| pattern == "=> ") {
        log.push_str(&format!(
            "\n# bootswain waiting for U-Boot prompt for {}\n",
            step.name
        ));
    }

    let mut step_segment = String::new();
    match read_until_patterns(
        session,
        ReadOptions {
            timeout,
            idle_sleep: config.idle_sleep,
            progress_interval: config.progress_interval,
            progress_label: Some(step.name.clone()),
            fail_on_prompt: step.command.is_some(),
            repeat_prompt_request: false,
            reset_policy: ResetPolicy::Fail,
        },
        &config.cancellation,
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
    log.push_str(&format!(
        "\n# bootswain cleared stale serial input before command for {}\n",
        step.name
    ));
    session
        .clear()
        .with_context(|| format!("failed to clear serial input before step {}", step.name))
}

fn read_until_patterns<T>(
    session: &mut T,
    options: ReadOptions,
    cancellation: &AtomicBool,
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
    let mut last_progress = start;
    let mut last_prompt_request = start;
    let mut boot_menu_shell_selected = false;
    let mut buffer = [0_u8; 1024];
    while start.elapsed() < options.timeout {
        if cancellation.load(Ordering::SeqCst) {
            return Ok(ReadStatus::Interrupted);
        }
        let read = session.read_chunk(&mut buffer)?;
        if read == 0 {
            if options.progress_interval > Duration::ZERO
                && last_progress.elapsed() >= options.progress_interval
            {
                if let Some(label) = &options.progress_label {
                    eprintln!(
                        "bootswain: {label}: still waiting after {}s/{timeout_secs}s, captured {} byte(s)",
                        start.elapsed().as_secs(),
                        segment_log.len(),
                        timeout_secs = options.timeout.as_secs()
                    );
                }
                last_progress = Instant::now();
            }
            maybe_request_prompt(
                session,
                options.repeat_prompt_request,
                &mut last_prompt_request,
                &mut boot_menu_shell_selected,
                full_log,
                segment_log,
            )?;
            if !options.idle_sleep.is_zero() {
                thread::sleep(options.idle_sleep);
            }
            continue;
        }

        let chunk = String::from_utf8_lossy(&buffer[..read]);
        full_log.push_str(&chunk);
        segment_log.push_str(&chunk);

        if patterns.iter().all(|pattern| segment_log.contains(pattern)) {
            return Ok(ReadStatus::Matched);
        }
        maybe_request_prompt(
            session,
            options.repeat_prompt_request,
            &mut last_prompt_request,
            &mut boot_menu_shell_selected,
            full_log,
            segment_log,
        )?;
        if rockpro64_serial_segment_has_reset(segment_log, options.reset_policy) {
            return Ok(ReadStatus::Reset);
        }
        if options.fail_on_prompt
            && let Some(marker) = terminal_failure_marker(segment_log)
        {
            return Ok(ReadStatus::Failure(format!(
                "terminal serial failure before expected patterns appeared: {marker:?}; waiting for {}",
                format_expected_patterns(patterns)
            )));
        }
        if options.fail_on_prompt && segment_has_uboot_prompt(segment_log) {
            return Ok(ReadStatus::Failure(format!(
                "command returned to U-Boot prompt before expected serial patterns appeared: {}",
                format_expected_patterns(patterns)
            )));
        }
    }

    Ok(ReadStatus::Timeout)
}

fn maybe_request_prompt<T>(
    session: &mut T,
    enabled: bool,
    last_prompt_request: &mut Instant,
    boot_menu_shell_selected: &mut bool,
    log: &mut String,
    segment_log: &str,
) -> Result<()>
where
    T: SerialIo,
{
    if !enabled {
        return Ok(());
    }

    if segment_has_boot_menu_shell_entry(segment_log) && !*boot_menu_shell_selected {
        log.push_str(
            "\n# bootswain selected U-Boot shell from boot menu while waiting for prompt\n",
        );
        session
            .write_all(b"9\n")
            .context("failed to select U-Boot shell from boot menu")?;
        session
            .flush()
            .context("failed to flush boot menu shell selection")?;
        *last_prompt_request = Instant::now();
        *boot_menu_shell_selected = true;
        return Ok(());
    }

    if last_prompt_request.elapsed() < Duration::from_secs(2) {
        return Ok(());
    }

    log.push_str("\n# bootswain sent autoboot interrupt while waiting for U-Boot prompt\n");
    session
        .write_all(b"\n")
        .context("failed to send autoboot interrupt")?;
    session
        .flush()
        .context("failed to flush autoboot interrupt")?;
    *last_prompt_request = Instant::now();
    Ok(())
}

fn effective_step_timeout(step: &ValidationStep, config: &RockPro64SerialRunConfig) -> Duration {
    let plan_timeout = Duration::from_secs(step.timeout_secs);
    match config.max_step_timeout {
        Some(max_timeout) if max_timeout < plan_timeout => max_timeout,
        _ => plan_timeout,
    }
}

fn selected_timeout_secs(
    plan: &ValidationPlan,
    selected: &BTreeSet<String>,
    config: &RockPro64SerialRunConfig,
) -> u64 {
    plan.scenarios
        .iter()
        .filter(|scenario| selected.contains(&scenario.name))
        .flat_map(|scenario| scenario.steps.iter())
        .map(|step| effective_step_timeout(step, config).as_secs())
        .sum()
}

fn progressln(config: &RockPro64SerialRunConfig, message: &str) {
    if config.progress_interval > Duration::ZERO {
        eprintln!("bootswain: {message}");
    }
}

fn rockpro64_serial_segment_has_reset(text: &str, reset_policy: ResetPolicy) -> bool {
    let hard_reset_markers = ["Synchronous Abort", "Resetting CPU", "resetting ..."];
    if hard_reset_markers
        .iter()
        .any(|needle| text.contains(needle))
    {
        return true;
    }

    if reset_policy == ResetPolicy::IgnoreBootBanners {
        return false;
    }

    ["\nU-Boot TPL ", "\nU-Boot SPL ", "\nU-Boot "]
        .iter()
        .any(|needle| text.contains(needle))
}

fn terminal_failure_marker(text: &str) -> Option<&'static str> {
    [
        "SPI probe failed",
        "SPI flash write failed",
        "SPI verify mismatch",
        "SPI verify read failed",
        "SPI erase command failed",
        "SPI flashing is refused from installed SPI firmware",
        "BootsWain installer media detected under installed firmware",
        "Failed to load SPI payload",
        "Unexpected board compatible",
        "Boot failed (err=",
        "No more bootdevs",
        "No detected boot options",
        "No bootable media on",
    ]
    .into_iter()
    .find(|marker| text.contains(marker))
}

fn segment_has_uboot_prompt(text: &str) -> bool {
    text.starts_with("=> ") || text.contains("\n=> ") || text.ends_with("=> ")
}

fn segment_has_boot_menu_shell_entry(text: &str) -> bool {
    text.contains("*** U-Boot Boot Menu ***") && text.contains("Enter U-Boot shell")
}

fn format_expected_patterns(patterns: &[String]) -> String {
    patterns
        .iter()
        .map(|pattern| format!("{pattern:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_destructive_spi_scenario(scenario: &ValidationScenario) -> bool {
    matches!(
        scenario.kind,
        ValidationScenarioKind::SpiInstall | ValidationScenarioKind::SpiErase
    )
}

fn command_step_allows_precommand_match(scenario: &ValidationScenario) -> bool {
    !is_destructive_spi_scenario(scenario)
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
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;
    use tempfile::tempdir;

    #[derive(Debug, Default)]
    struct FakeSerial {
        chunks: VecDeque<Vec<u8>>,
        writes: Vec<String>,
        clear_drops: VecDeque<usize>,
    }

    impl FakeSerial {
        fn with_chunks(chunks: &[&str]) -> Self {
            Self {
                chunks: chunks
                    .iter()
                    .map(|chunk| chunk.as_bytes().to_vec())
                    .collect(),
                writes: Vec::new(),
                clear_drops: VecDeque::new(),
            }
        }

        fn with_chunks_and_clear_drops(chunks: &[&str], clear_drops: &[usize]) -> Self {
            Self {
                chunks: chunks
                    .iter()
                    .map(|chunk| chunk.as_bytes().to_vec())
                    .collect(),
                writes: Vec::new(),
                clear_drops: clear_drops.iter().copied().collect(),
            }
        }
    }

    impl SerialIo for FakeSerial {
        fn clear(&mut self) -> io::Result<()> {
            let Some(drop_count) = self.clear_drops.pop_front() else {
                return Ok(());
            };
            for _ in 0..drop_count {
                let _ = self.chunks.pop_front();
            }
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
    fn command_step_catches_autoboot_after_reset_before_write() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
        let mut serial = FakeSerial::with_chunks(&[
            "\nU-Boot TPL 2026.04\n",
            "U-Boot SPL 2026.04\n",
            "U-Boot 2026.04\nHit any key to stop autoboot:  0\n=> ",
            "Scanning for bootflows\n=> ",
        ]);

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
    fn command_step_selects_shell_from_boot_menu_before_write() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
        let mut serial = FakeSerial::with_chunks(&[
            "*** U-Boot Boot Menu ***\n1. Continue boot\n9. Enter U-Boot shell\n",
            "=> ",
            "Scanning for bootflows\n=> ",
        ]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "bootflow")
            .expect("bootflow outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
        assert_eq!(serial.writes, vec!["9\n", "bootflow scan\n"]);
    }

    #[test]
    fn non_destructive_command_step_passes_when_expected_output_appears_before_prompt() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
        let mut serial = FakeSerial::with_chunks(&["Scanning for bootflows\n=> "]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "bootflow")
            .expect("bootflow outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
        assert!(serial.writes.is_empty());
    }

    #[test]
    fn command_step_clears_stale_input_after_prompt_before_write() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
        let mut serial = FakeSerial::with_chunks_and_clear_drops(
            &[
                "=> ",
                "SPI probe failed\n",
                "Flashing SPI payload from mmc 0:1\nSPI flash complete\n",
            ],
            &[0, 1],
        );

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
    fn boot_banner_after_command_marks_selected_scenario_failed() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
        let mut serial = FakeSerial::with_chunks(&[
            "=> ",
            "Flashing SPI payload from mmc 1:1\nWriting SPI payload\n\nU-Boot TPL 2026.04\n",
        ]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
        assert!(
            outcome
                .failure
                .as_deref()
                .expect("failure")
                .contains("board reset")
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
        let mut serial = FakeSerial::with_chunks(&[
            "=> ",
            "Flashing SPI payload from mmc 0:1\nSPI flash complete\n",
        ]);

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
    fn terminal_failure_marker_fails_without_waiting_for_full_timeout() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
        let mut serial = FakeSerial::with_chunks(&[
            "=> ",
            "Flashing SPI payload from mmc 0:1\nSPI probe failed\n=> ",
        ]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
        assert!(
            outcome
                .failure
                .as_deref()
                .expect("failure")
                .contains("SPI probe failed")
        );
    }

    #[test]
    fn installed_firmware_spi_refusal_fails_without_waiting_for_full_timeout() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
        let mut serial = FakeSerial::with_chunks(&[
            "=> ",
            "SPI flashing is refused from installed SPI firmware\n=> ",
        ]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
        assert!(
            outcome
                .failure
                .as_deref()
                .expect("failure")
                .contains("refused from installed SPI firmware")
        );
    }

    #[test]
    fn expected_no_bootable_media_marker_still_passes() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
        let mut plan = sample_plan();
        plan.scenarios[1].steps[0].expect = vec!["No bootable media on SD".into()];
        let mut serial = FakeSerial::with_chunks(&["=> ", "No bootable media on SD\n=> "]);

        let run = run_rockpro64_serial_validation_with_session(plan, &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "bootflow")
            .expect("bootflow outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
    }

    #[test]
    fn prompt_return_after_command_without_expected_patterns_fails_immediately() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
        let mut serial =
            FakeSerial::with_chunks(&["=> ", "Flashing SPI payload from mmc 0:1\n=> "]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let outcome = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
        assert!(
            outcome
                .failure
                .as_deref()
                .expect("failure")
                .contains("command returned to U-Boot prompt")
        );
    }

    #[test]
    fn selected_run_stops_after_first_failed_scenario() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(
            tmp.path().join("out"),
            vec!["bootflow".into(), "spi-install".into()],
            true,
        );
        let mut serial = FakeSerial::with_chunks(&["=> ", "SPI probe failed\n=> "]);

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let bootflow = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "bootflow")
            .expect("bootflow outcome");
        let spi = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "spi-install")
            .expect("spi outcome");
        assert_eq!(bootflow.status, ValidationOutcomeStatus::Failed);
        assert_eq!(spi.status, ValidationOutcomeStatus::NotRun);
        assert_eq!(
            spi.failure.as_deref(),
            Some("run stopped after failed scenario: bootflow")
        );
    }

    #[test]
    fn interruption_writes_partial_run_and_marks_selected_scenarios_interrupted() {
        let tmp = tempdir().expect("tempdir");
        let config = sample_config(
            tmp.path().join("out"),
            vec!["recovery-console".into(), "bootflow".into()],
            false,
        );
        config.cancellation.store(true, Ordering::SeqCst);
        let mut serial = FakeSerial::default();

        let run = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
            .expect("serial validation");

        let recovery = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "recovery-console")
            .expect("recovery outcome");
        let bootflow = run
            .outcomes
            .iter()
            .find(|outcome| outcome.scenario == "bootflow")
            .expect("bootflow outcome");
        assert_eq!(recovery.status, ValidationOutcomeStatus::Interrupted);
        assert_eq!(bootflow.status, ValidationOutcomeStatus::Interrupted);
        assert!(config.out.join("recovery-console/serial.log").exists());
        assert!(config.out.join("validation-run.json").exists());
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
            progress_interval: Duration::ZERO,
            max_step_timeout: None,
            cancellation: Arc::new(AtomicBool::new(false)),
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
                        expect: vec![
                            "Flashing SPI payload from".into(),
                            "SPI flash complete".into(),
                        ],
                        timeout_secs: 1,
                    }],
                },
            ],
        }
    }
}
