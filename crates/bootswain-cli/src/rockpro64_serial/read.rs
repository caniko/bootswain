use anyhow::{Context, Result};
use bootswain_probe::SerialIo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReadStatus {
    Matched,
    Reset,
    Failure(String),
    Interrupted,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResetPolicy {
    Fail,
    IgnoreBootBanners,
}

#[derive(Debug, Clone)]
pub(super) struct ReadOptions {
    pub(super) timeout: Duration,
    pub(super) idle_sleep: Duration,
    pub(super) progress_interval: Duration,
    pub(super) progress_label: Option<String>,
    pub(super) fail_on_prompt: bool,
    pub(super) repeat_prompt_request: bool,
    pub(super) reset_policy: ResetPolicy,
}

pub(super) fn read_until_patterns<T>(
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

pub(super) fn format_expected_patterns(patterns: &[String]) -> String {
    patterns
        .iter()
        .map(|pattern| format!("{pattern:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}
