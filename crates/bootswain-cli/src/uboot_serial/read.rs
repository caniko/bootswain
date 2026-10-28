use anyhow::{Context, Result};
use bootswain_probe::{
    MAX_MARKER_LEN, SerialIo, Utf8Accumulator, rescan_start, segment_has_boot_banner,
    segment_has_prompt_in, segment_has_reset,
};
use std::ops::Range;
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

/// Result of one read: why it stopped, and where in the log the device bytes of
/// this read live. The log also holds bootswain's own `# ...` annotations, which
/// must never satisfy a pattern, so the segment is a list of ranges rather than
/// one slice.
#[derive(Debug)]
pub(super) struct ReadOutcome {
    pub(super) status: ReadStatus,
    runs: Vec<Range<usize>>,
}

impl ReadOutcome {
    /// The device text captured by this read, without bootswain annotations.
    pub(super) fn segment(&self, log: &str) -> String {
        self.runs.iter().map(|run| &log[run.clone()]).collect()
    }
}

const TERMINAL_FAILURE_MARKERS: [&str; 13] = [
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
];
const BOOT_MENU_HEADER: &str = "*** U-Boot Boot Menu ***";
const BOOT_MENU_SHELL_ENTRY: &str = "Enter U-Boot shell";

/// Incremental matcher state: everything here is monotonic over a segment (once a
/// needle has appeared it stays appeared), so each chunk only needs its own window.
struct Scan<'a> {
    patterns: &'a [String],
    found: Vec<bool>,
    menu_header: bool,
    menu_entry: bool,
    overlap: usize,
}

impl<'a> Scan<'a> {
    fn new(patterns: &'a [String]) -> Self {
        let longest = patterns
            .iter()
            .map(String::len)
            .chain(TERMINAL_FAILURE_MARKERS.iter().map(|marker| marker.len()))
            .chain([MAX_MARKER_LEN, BOOT_MENU_HEADER.len()])
            .max()
            .unwrap_or(MAX_MARKER_LEN);
        Self {
            patterns,
            found: vec![false; patterns.len()],
            menu_header: false,
            menu_entry: false,
            overlap: longest - 1,
        }
    }

    fn update(&mut self, window: &str) {
        for (found, pattern) in self.found.iter_mut().zip(self.patterns) {
            *found = *found || window.contains(pattern.as_str());
        }
        self.menu_header = self.menu_header || window.contains(BOOT_MENU_HEADER);
        self.menu_entry = self.menu_entry || window.contains(BOOT_MENU_SHELL_ENTRY);
    }

    fn all_found(&self) -> bool {
        self.found.iter().all(|found| *found)
    }

    fn boot_menu_shell_entry(&self) -> bool {
        self.menu_header && self.menu_entry
    }
}

/// Reads serial data into `log` until every pattern has appeared, the board resets,
/// a failure is detected, the read is cancelled, or the timeout elapses.
///
/// Per chunk only the newly decoded text plus a short overlap is scanned, so total
/// work is linear in the captured output.
pub(super) fn read_until_patterns<T>(
    session: &mut T,
    options: ReadOptions,
    cancellation: &AtomicBool,
    log: &mut String,
    patterns: &[String],
) -> Result<ReadOutcome>
where
    T: SerialIo,
{
    let mut runs: Vec<Range<usize>> = Vec::new();
    if patterns.is_empty() {
        return Ok(ReadOutcome {
            status: ReadStatus::Matched,
            runs,
        });
    }

    let start = Instant::now();
    let mut last_progress = start;
    let mut last_prompt_request = start;
    let mut boot_menu_shell_selected = false;
    let mut captured = 0_usize;
    let mut scan = Scan::new(patterns);
    let mut decoder = Utf8Accumulator::default();
    let mut buffer = [0_u8; 1024];
    let finish = |status, runs| Ok(ReadOutcome { status, runs });
    while start.elapsed() < options.timeout {
        if cancellation.load(Ordering::SeqCst) {
            return finish(ReadStatus::Interrupted, runs);
        }
        let read = session.read_chunk(&mut buffer)?;
        if read == 0 {
            if options.progress_interval > Duration::ZERO
                && last_progress.elapsed() >= options.progress_interval
            {
                if let Some(label) = &options.progress_label {
                    eprintln!(
                        "bootswain: {label}: still waiting after {}s/{timeout_secs}s, captured {captured} byte(s)",
                        start.elapsed().as_secs(),
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
                scan.boot_menu_shell_entry(),
                log,
            )?;
            if !options.idle_sleep.is_zero() {
                thread::sleep(options.idle_sleep);
            }
            continue;
        }

        let appended_from = log.len();
        decoder.push(&buffer[..read], log);
        captured += log.len() - appended_from;
        if log.len() > appended_from {
            match runs.last_mut() {
                Some(run) if run.end == appended_from => run.end = log.len(),
                _ => runs.push(appended_from..log.len()),
            }
        }

        // Window = this chunk plus overlap, kept inside the current run of device
        // bytes so annotations are never scanned.
        let run_start = runs.last().map_or(appended_from, |run| run.start);
        let from = rescan_start(log, run_start, appended_from, scan.overlap);
        let window = &log[from..];
        scan.update(window);
        if scan.all_found() {
            return finish(ReadStatus::Matched, runs);
        }

        // Evaluate on the window before maybe_request_prompt appends an annotation.
        let at_segment_start = runs.len() == 1 && from == runs[0].start;
        let reset = segment_has_reset(window)
            || (options.reset_policy == ResetPolicy::Fail && segment_has_boot_banner(window));
        let failure_marker = terminal_failure_marker(window).filter(|_| options.fail_on_prompt);
        let at_prompt = options.fail_on_prompt && segment_has_prompt_in(window, at_segment_start);

        maybe_request_prompt(
            session,
            options.repeat_prompt_request,
            &mut last_prompt_request,
            &mut boot_menu_shell_selected,
            scan.boot_menu_shell_entry(),
            log,
        )?;
        if reset {
            return finish(ReadStatus::Reset, runs);
        }
        if let Some(marker) = failure_marker {
            return finish(
                ReadStatus::Failure(format!(
                    "terminal serial failure before expected patterns appeared: {marker:?}; waiting for {}",
                    format_expected_patterns(patterns)
                )),
                runs,
            );
        }
        if at_prompt {
            return finish(
                ReadStatus::Failure(format!(
                    "command returned to U-Boot prompt before expected serial patterns appeared: {}",
                    format_expected_patterns(patterns)
                )),
                runs,
            );
        }
    }

    finish(ReadStatus::Timeout, runs)
}

fn maybe_request_prompt<T>(
    session: &mut T,
    enabled: bool,
    last_prompt_request: &mut Instant,
    boot_menu_shell_selected: &mut bool,
    boot_menu_shell_entry_seen: bool,
    log: &mut String,
) -> Result<()>
where
    T: SerialIo,
{
    if !enabled {
        return Ok(());
    }

    if boot_menu_shell_entry_seen && !*boot_menu_shell_selected {
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

fn terminal_failure_marker(text: &str) -> Option<&'static str> {
    TERMINAL_FAILURE_MARKERS
        .into_iter()
        .find(|marker| text.contains(marker))
}

pub(super) fn format_expected_patterns(patterns: &[String]) -> String {
    patterns
        .iter()
        .map(|pattern| format!("{pattern:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io;

    struct Chunks(VecDeque<Vec<u8>>);

    impl SerialIo for Chunks {
        fn clear(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let Some(chunk) = self.0.pop_front() else {
                return Ok(0);
            };
            buffer[..chunk.len()].copy_from_slice(&chunk);
            Ok(chunk.len())
        }

        fn write_all(&mut self, _buffer: &[u8]) -> io::Result<()> {
            Ok(())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn options() -> ReadOptions {
        ReadOptions {
            timeout: Duration::from_millis(50),
            idle_sleep: Duration::ZERO,
            progress_interval: Duration::ZERO,
            progress_label: None,
            fail_on_prompt: false,
            repeat_prompt_request: false,
            reset_policy: ResetPolicy::Fail,
        }
    }

    fn read(chunks: &[&[u8]], patterns: &[&str]) -> (ReadStatus, String) {
        let mut serial = Chunks(chunks.iter().map(|chunk| chunk.to_vec()).collect());
        let mut log = String::new();
        let patterns: Vec<String> = patterns.iter().map(|p| (*p).to_owned()).collect();
        let outcome = read_until_patterns(
            &mut serial,
            options(),
            &AtomicBool::new(false),
            &mut log,
            &patterns,
        )
        .expect("read");
        (outcome.status, log)
    }

    #[test]
    fn pattern_split_across_chunks_matches() {
        let (status, _) = read(&[b"Start", b"ing ker", b"nel\n"], &["Starting kernel"]);
        assert_eq!(status, ReadStatus::Matched);
    }

    #[test]
    fn reset_marker_split_across_chunks_is_detected() {
        let (status, _) = read(&[b"boot Resett", b"ing CPU ...", b"=> "], &["never"]);
        assert_eq!(status, ReadStatus::Reset);
    }

    #[test]
    fn multibyte_character_split_across_chunks_is_not_corrupted() {
        let bytes = "caf\u{e9}\u{2713} ok".as_bytes();
        let chunks: Vec<&[u8]> = bytes.chunks(1).collect();
        let (status, log) = read(&chunks, &["\u{2713} ok"]);
        assert_eq!(status, ReadStatus::Matched);
        assert_eq!(log, "caf\u{e9}\u{2713} ok");
    }

    #[test]
    fn failure_markers_fit_inside_the_rescan_overlap() {
        for marker in TERMINAL_FAILURE_MARKERS {
            assert!(Scan::new(&[]).overlap >= marker.len() - 1);
        }
    }
}
