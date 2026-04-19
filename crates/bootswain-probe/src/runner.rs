use crate::parser::{
    classify_autoboot, extract_last_usb_tree_summary, segment_has_prompt, segment_has_reset,
};
use crate::serial::{RealSerial, SerialIo};
use anyhow::{Context, Result, bail};
use bootswain_core::{Board, ImageInfo, ProbeStepResult, SummaryResult, TrialResult};
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

    println!("ROCKPro64 UART caveat:");
    println!("  - connect only GND + board TX during power-on");
    println!("  - leave board RX / pin 10 disconnected until U-Boot is already up");
    println!();

    let mut trials = Vec::with_capacity(config.repeat as usize);
    for trial_number in 1..=config.repeat {
        prompt_operator(trial_number, config.repeat)?;

        let trial_dir = config.out_dir.join(format!("trial-{trial_number}"));
        fs::create_dir_all(&trial_dir)
            .with_context(|| format!("failed to create {}", trial_dir.display()))?;

        let mut serial = RealSerial::open(&config.port, config.baud)?;
        let trial = run_trial_with_session(&mut serial, config, trial_number, &trial_dir)?;
        trials.push(trial);
    }

    let summary = SummaryResult::from_trials(&config.image, Board::RockPro64, trials);
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

    let mut log = String::new();
    let autoboot =
        read_until_terminator(session, config.prompt_timeout, config.idle_sleep, &mut log)?;
    let autoboot_result = classify_autoboot(&autoboot);
    let autoboot_terminated = classify_termination(&autoboot);

    let mut usb_tree_result = ProbeStepResult::NotRun;
    let mut usb_reset_result = ProbeStepResult::NotRun;
    let mut final_detected_usb_summary = String::new();

    if autoboot_terminated == SegmentTermination::Prompt {
        let first_tree = run_command(
            session,
            "usb tree",
            config.command_timeout,
            config.idle_sleep,
            &mut log,
        )?;
        usb_tree_result = first_tree.result;
        if first_tree.result == ProbeStepResult::Ok {
            final_detected_usb_summary = extract_last_usb_tree_summary(&first_tree.output);

            let usb_reset = run_command(
                session,
                "usb reset",
                config.command_timeout,
                config.idle_sleep,
                &mut log,
            )?;
            usb_reset_result = usb_reset.result;

            if usb_reset.result == ProbeStepResult::Ok {
                let final_tree = run_command(
                    session,
                    "usb tree",
                    config.command_timeout,
                    config.idle_sleep,
                    &mut log,
                )?;
                usb_tree_result = final_tree.result;
                if final_tree.result == ProbeStepResult::Ok {
                    let summary = extract_last_usb_tree_summary(&final_tree.output);
                    if !summary.is_empty() {
                        final_detected_usb_summary = summary;
                    }
                }
            }
        }
    }

    let raw_log_path = trial_dir.join("serial.log");
    fs::write(&raw_log_path, &log)
        .with_context(|| format!("failed to write {}", raw_log_path.display()))?;

    let trial = TrialResult {
        image_path: config.image.path.clone(),
        image_sha256: config.image.sha256.clone(),
        board: Board::RockPro64,
        serial_port: config.port.clone(),
        baud: config.baud,
        trial_number,
        autoboot_result,
        usb_tree_result,
        usb_reset_result,
        final_detected_usb_summary,
        raw_log_path: raw_log_path.clone(),
    };

    let trial_json_path = trial_dir.join("trial.json");
    let trial_json = serde_json::to_string_pretty(&trial).context("failed to encode trial")?;
    fs::write(&trial_json_path, trial_json)
        .with_context(|| format!("failed to write {}", trial_json_path.display()))?;

    Ok(trial)
}

fn prompt_operator(trial_number: u32, total_trials: u32) -> Result<()> {
    println!("Trial {trial_number}/{total_trials}");
    println!("Press Enter to start capture, then power-cycle the board immediately.");
    print!("> ");
    io::stdout().flush().context("failed to flush stdout")?;

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("failed to read operator confirmation")?;

    Ok(())
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
