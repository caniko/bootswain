use crate::parser::{
    classify_autoboot, extract_last_usb_tree_summary, segment_has_prompt, segment_has_reset,
};
use crate::profile::{ProbeProfile, ProbeResultField, rockpro64_usb_profile};
use crate::serial::{RealSerial, SerialIo};
use anyhow::{Context, Result, bail};
use bootswain_core::{FailureStage, ImageInfo, ProbeStepResult, SummaryResult, TrialResult};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ProbeConfig {
    pub image: ImageInfo,
    pub port: String,
    pub baud: u32,
    pub out_dir: PathBuf,
    pub repeat: u32,
    pub prompt_timeout: Duration,
    pub command_timeout: Duration,
    pub idle_sleep: Duration,
    pub progress_to_stderr: bool,
    profile: ProbeProfile,
}

impl ProbeConfig {
    #[must_use]
    pub fn new(image: ImageInfo, port: String, baud: u32, out_dir: PathBuf, repeat: u32) -> Self {
        Self {
            image,
            port,
            baud,
            out_dir,
            repeat,
            prompt_timeout: Duration::from_secs(20),
            command_timeout: Duration::from_secs(10),
            idle_sleep: Duration::from_millis(20),
            progress_to_stderr: false,
            profile: rockpro64_usb_profile(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentTermination {
    Prompt,
    Reset,
    Timeout,
}

#[derive(Debug)]
struct CommandOutcome {
    result: ProbeStepResult,
    output: String,
}

pub fn run_rockpro64_usb_probe(config: &ProbeConfig) -> Result<SummaryResult> {
    if config.repeat == 0 {
        bail!("repeat must be at least 1");
    }

    fs::create_dir_all(&config.out_dir)
        .with_context(|| format!("failed to create {}", config.out_dir.display()))?;

    progressln(config, "ROCKPro64 UART caveat:");
    progressln(config, "  - connect only GND + board TX during power-on");
    progressln(
        config,
        "  - leave board RX / pin 10 disconnected until U-Boot is already up",
    );
    progressln(config, "");

    let mut trials = Vec::with_capacity(config.repeat as usize);
    for trial_number in 1..=config.repeat {
        prompt_operator(config, trial_number, config.repeat)?;

        let trial_dir = config.out_dir.join(format!("trial-{trial_number}"));
        fs::create_dir_all(&trial_dir)
            .with_context(|| format!("failed to create {}", trial_dir.display()))?;

        let mut serial = RealSerial::open(&config.port, config.baud)?;
        let trial = run_trial_with_session(&mut serial, config, trial_number, &trial_dir)?;
        progressln(
            config,
            &format!(
                "  final: {}",
                if trial.is_clean() { "clean" } else { "failed" }
            ),
        );
        progressln(config, &format!("  failure-stage: {}", trial.failure_stage));
        if !trial.final_detected_usb_summary.is_empty() {
            progressln(
                config,
                &format!(
                    "  detected-usb: {}",
                    trial.final_detected_usb_summary.replace('\n', " | ")
                ),
            );
        }
        progressln(config, "");
        trials.push(trial);
    }

    let summary = SummaryResult::from_trials(&config.image, config.profile.board, trials);
    let summary_path = config.out_dir.join("summary.json");
    let summary_json =
        serde_json::to_string_pretty(&summary).context("failed to encode summary")?;
    fs::write(&summary_path, summary_json)
        .with_context(|| format!("failed to write {}", summary_path.display()))?;

    Ok(summary)
}

pub fn run_trial_with_session<T: SerialIo>(
    session: &mut T,
    config: &ProbeConfig,
    trial_number: u32,
    trial_dir: &Path,
) -> Result<TrialResult> {
    let _ = session.clear();

    progressln(config, "  waiting for U-Boot prompt...");
    let mut log = String::new();
    let autoboot =
        read_until_terminator(session, config.prompt_timeout, config.idle_sleep, &mut log)?;
    let autoboot_result = classify_autoboot(&autoboot);
    let autoboot_terminated = classify_termination(&autoboot);
    progressln(
        config,
        &format!("  autoboot: {}", format_autoboot(autoboot_result)),
    );

    let mut trial = TrialResult {
        image_path: config.image.path.clone(),
        image_sha256: config.image.sha256.clone(),
        board: config.profile.board,
        serial_port: config.port.clone(),
        baud: config.baud,
        trial_number,
        autoboot_result,
        usb_start_result: ProbeStepResult::NotRun,
        usb_tree_result: ProbeStepResult::NotRun,
        usb_reset_result: ProbeStepResult::NotRun,
        failure_stage: FailureStage::None,
        final_detected_usb_summary: String::new(),
        raw_log_path: trial_dir.join("serial.log"),
    };

    if autoboot_terminated != SegmentTermination::Prompt {
        trial.failure_stage = FailureStage::Autoboot;
        progressln(
            config,
            &format!(
                "  autoboot-stage: {}",
                match autoboot_terminated {
                    SegmentTermination::Prompt => "prompt",
                    SegmentTermination::Reset => "reset",
                    SegmentTermination::Timeout => "timeout",
                }
            ),
        );
        persist_trial(trial_dir, &log, &trial)?;
        return Ok(trial);
    }

    for step in config.profile.steps {
        progressln(config, &format!("  {}...", step.stage));
        let outcome = run_command(
            session,
            step.command,
            config.command_timeout,
            config.idle_sleep,
            &mut log,
        )?;
        set_probe_result(&mut trial, step.result_field, outcome.result);

        if matches!(step.stage, FailureStage::UsbTree1 | FailureStage::UsbTree2)
            && outcome.result == ProbeStepResult::Ok
        {
            let summary = extract_last_usb_tree_summary(&outcome.output);
            if !summary.is_empty() {
                trial.final_detected_usb_summary = summary;
            }
        }

        progressln(config, &format!("  {}: {}", step.stage, outcome.result));
        if outcome.result != ProbeStepResult::Ok {
            trial.failure_stage = step.stage;
            persist_trial(trial_dir, &log, &trial)?;
            return Ok(trial);
        }
    }

    persist_trial(trial_dir, &log, &trial)?;
    Ok(trial)
}

fn prompt_operator(config: &ProbeConfig, trial_number: u32, total_trials: u32) -> Result<()> {
    progressln(config, &format!("Trial {trial_number}/{total_trials}"));
    progressln(config, &format!("  image: {}", config.image.path.display()));
    progressln(
        config,
        &format!("  port: {} @ {}", config.port, config.baud),
    );
    progressln(
        config,
        &format!("  prompt-timeout-secs: {}", config.prompt_timeout.as_secs()),
    );
    progressln(
        config,
        &format!(
            "  command-timeout-secs: {}",
            config.command_timeout.as_secs()
        ),
    );
    progressln(
        config,
        "Press Enter to start capture, then power-cycle the board immediately.",
    );
    progress_prompt(config, "> ")?;

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("failed to read operator confirmation")?;

    Ok(())
}

fn progressln(config: &ProbeConfig, message: &str) {
    if config.progress_to_stderr {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}

fn progress_prompt(config: &ProbeConfig, message: &str) -> Result<()> {
    if config.progress_to_stderr {
        eprint!("{message}");
        io::stderr().flush().context("failed to flush stderr")?;
    } else {
        print!("{message}");
        io::stdout().flush().context("failed to flush stdout")?;
    }
    Ok(())
}

fn persist_trial(trial_dir: &Path, log: &str, trial: &TrialResult) -> Result<()> {
    fs::write(&trial.raw_log_path, log)
        .with_context(|| format!("failed to write {}", trial.raw_log_path.display()))?;

    let trial_json_path = trial_dir.join("trial.json");
    let trial_json = serde_json::to_string_pretty(trial).context("failed to encode trial")?;
    fs::write(&trial_json_path, trial_json)
        .with_context(|| format!("failed to write {}", trial_json_path.display()))?;
    Ok(())
}

fn format_autoboot(result: bootswain_core::AutobootResult) -> &'static str {
    match result {
        bootswain_core::AutobootResult::Abort => "abort",
        bootswain_core::AutobootResult::WarnEnumerate => "warn+enumerate",
        bootswain_core::AutobootResult::NoDevice => "no-device",
        bootswain_core::AutobootResult::Other => "other",
    }
}

fn set_probe_result(trial: &mut TrialResult, field: ProbeResultField, result: ProbeStepResult) {
    match field {
        ProbeResultField::Start => trial.usb_start_result = result,
        ProbeResultField::Tree => trial.usb_tree_result = result,
        ProbeResultField::Reset => trial.usb_reset_result = result,
    }
}

fn classify_termination(text: &str) -> SegmentTermination {
    if segment_has_reset(text) {
        SegmentTermination::Reset
    } else if segment_has_prompt(text) {
        SegmentTermination::Prompt
    } else {
        SegmentTermination::Timeout
    }
}

fn run_command<T: SerialIo>(
    session: &mut T,
    command: &str,
    timeout: Duration,
    idle_sleep: Duration,
    full_log: &mut String,
) -> Result<CommandOutcome> {
    session
        .write_all(format!("{command}\n").as_bytes())
        .with_context(|| format!("failed to send command {command}"))?;
    session
        .flush()
        .with_context(|| format!("failed to flush command {command}"))?;

    let output = read_until_terminator(session, timeout, idle_sleep, full_log)?;
    let result = match classify_termination(&output) {
        SegmentTermination::Prompt => ProbeStepResult::Ok,
        SegmentTermination::Reset => ProbeStepResult::Reset,
        SegmentTermination::Timeout => ProbeStepResult::Timeout,
    };

    Ok(CommandOutcome { result, output })
}

fn read_until_terminator<T: SerialIo>(
    session: &mut T,
    timeout: Duration,
    idle_sleep: Duration,
    full_log: &mut String,
) -> Result<String> {
    let start_len = full_log.len();
    let start = Instant::now();
    let mut buffer = [0_u8; 4096];

    loop {
        let read = session
            .read_chunk(&mut buffer)
            .context("failed to read serial data")?;
        if read > 0 {
            let chunk = String::from_utf8_lossy(&buffer[..read]);
            full_log.push_str(&chunk);
            let segment = &full_log[start_len..];
            if segment_has_reset(segment) || segment_has_prompt(segment) {
                return Ok(segment.to_owned());
            }
            continue;
        }

        if start.elapsed() >= timeout {
            return Ok(full_log[start_len..].to_owned());
        }

        thread::sleep(idle_sleep);
    }
}
