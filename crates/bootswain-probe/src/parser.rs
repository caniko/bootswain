use bootswain_core::AutobootResult;

pub fn classify_autoboot(text: &str) -> AutobootResult {
    if segment_has_reset(text) {
        return AutobootResult::Abort;
    }

    let lowered = text.to_ascii_lowercase();
    if (lowered.contains("warn ") || lowered.contains("error in inquiry"))
        && (lowered.contains("storage device(s) found") || lowered.contains("mass storage"))
    {
        AutobootResult::WarnEnumerate
    } else if lowered.contains("0 storage device(s) found") {
        AutobootResult::NoDevice
    } else {
        AutobootResult::Other
    }
}

pub fn segment_has_reset(text: &str) -> bool {
    [
        "Synchronous Abort",
        "Resetting CPU",
        "resetting ...",
        "Returning to boot ROM",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub fn segment_has_prompt(text: &str) -> bool {
    segment_has_prompt_in(text, true)
}

/// Shell prompts recognized without knowing the board: the default `=> ` and the
/// `U-Boot> ` that nixpkgs' Raspberry Pi builds print.
const PROMPTS: [&str; 2] = ["=> ", "U-Boot> "];

/// Prompt check for a rescan window of a longer segment; `at_segment_start` says whether
/// the window begins where the segment does (the only place a bare leading prompt counts).
pub fn segment_has_prompt_in(window: &str, at_segment_start: bool) -> bool {
    PROMPTS.iter().any(|prompt| {
        (at_segment_start && window.starts_with(prompt))
            || window
                .split('\n')
                .skip(1)
                .any(|line| line.starts_with(prompt))
            || window.ends_with(prompt)
            || window.ends_with(prompt.trim_end())
    })
}

/// A U-Boot stage banner at the start of a line, i.e. the board restarted its boot chain.
pub fn segment_has_boot_banner(text: &str) -> bool {
    ["\nU-Boot TPL ", "\nU-Boot SPL ", "\nU-Boot "]
        .iter()
        .any(|needle| text.contains(needle))
}

/// Longest needle any detector in this module looks for (bounds the rescan overlap).
pub const MAX_MARKER_LEN: usize = 24;

/// Where to resume scanning `text` after new bytes were appended at `appended_from`.
///
/// Goes back `overlap` bytes (longest needle minus one) so a needle split across two
/// chunks is still found, but never before `floor` (the start of the segment).
/// This keeps per-chunk work proportional to the chunk, not to the whole segment.
pub fn rescan_start(text: &str, floor: usize, appended_from: usize, overlap: usize) -> usize {
    let mut start = appended_from.saturating_sub(overlap).max(floor);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    start
}

/// Decodes serial bytes to text without splitting a multibyte character across reads.
#[derive(Debug, Default)]
pub struct Utf8Accumulator {
    pending: Vec<u8>,
}

impl Utf8Accumulator {
    /// Appends the decoded `bytes` to `out`; an incomplete trailing character is held back
    /// until the next call. Invalid bytes become U+FFFD like `String::from_utf8_lossy`.
    pub fn push(&mut self, bytes: &[u8], out: &mut String) {
        self.pending.extend_from_slice(bytes);
        let mut consumed = 0;
        loop {
            let rest = &self.pending[consumed..];
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    out.push_str(valid);
                    consumed = self.pending.len();
                    break;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    out.push_str(&String::from_utf8_lossy(&rest[..valid]));
                    consumed += valid;
                    let Some(bad) = error.error_len() else {
                        break;
                    };
                    out.push(char::REPLACEMENT_CHARACTER);
                    consumed += bad;
                }
            }
        }
        self.pending.drain(..consumed);
    }
}

pub fn extract_last_usb_tree_summary(text: &str) -> String {
    if let Some(index) = text.rfind("USB device tree:") {
        let tree_output = &text[index + "USB device tree:".len()..];
        let end = tree_output.find("\n=> ").unwrap_or(tree_output.len());
        let summary = tree_output[..end]
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned();

        if !summary.is_empty() {
            return summary;
        }
    }

    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty() && !line.trim_start().starts_with("=>"))
        .map(|line| line.trim().to_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_MARKER_LEN, Utf8Accumulator, classify_autoboot, extract_last_usb_tree_summary,
        rescan_start, segment_has_boot_banner, segment_has_prompt, segment_has_prompt_in,
        segment_has_reset,
    };
    use bootswain_core::AutobootResult;

    #[test]
    fn classifies_abort_and_warn_states() {
        assert_eq!(
            classify_autoboot(r#""Synchronous Abort" handler, esr 0x96000010"#),
            AutobootResult::Abort
        );
        assert_eq!(
            classify_autoboot("WARN urb submitted to disabled ep\n1 Storage Device(s) found"),
            AutobootResult::WarnEnumerate
        );
        assert_eq!(
            classify_autoboot("scanning usb for storage devices... 0 Storage Device(s) found"),
            AutobootResult::NoDevice
        );
    }

    #[test]
    fn detects_prompt_and_reset_markers() {
        assert!(segment_has_prompt("Hit any key to stop autoboot: 0\n=> "));
        assert!(segment_has_reset("Resetting CPU ..."));
    }

    #[test]
    fn prompt_window_only_counts_leading_prompt_at_segment_start() {
        assert!(segment_has_prompt_in("=> ", true));
        assert!(!segment_has_prompt_in("=> x", false));
        assert!(segment_has_prompt_in("x\n=> x", false));
        assert!(segment_has_prompt_in("boot =>", false));
        assert!(segment_has_prompt_in("U-Boot> ", true));
        assert!(segment_has_prompt_in("x\r\nU-Boot> ", false));
        assert!(segment_has_prompt_in("boot U-Boot>", false));
        assert!(!segment_has_prompt_in("U-Boot> x", false));
        assert!(segment_has_boot_banner("x\nU-Boot SPL 2026"));
        let markers = [
            "Synchronous Abort",
            "Resetting CPU",
            "resetting ...",
            "Returning to boot ROM",
        ];
        for marker in markers {
            assert!(marker.len() <= MAX_MARKER_LEN);
        }
    }

    #[test]
    fn rescan_window_finds_needle_split_across_chunks() {
        let mut log = String::from("boot Resetting");
        let appended_from = log.len();
        log.push_str(" CPU ...");
        assert!(!segment_has_reset(&log[appended_from..]));
        let from = rescan_start(&log, 0, appended_from, "Resetting CPU".len() - 1);
        assert!(segment_has_reset(&log[from..]));
    }

    #[test]
    fn rescan_start_stays_on_char_boundaries() {
        let log = "ééé=> ";
        let from = rescan_start(log, 0, log.len(), 3);
        assert!(log.is_char_boundary(from));
    }

    #[test]
    fn utf8_accumulator_keeps_split_characters_intact() {
        let bytes = "ok é\u{2713}".as_bytes();
        let mut text = String::new();
        let mut decoder = Utf8Accumulator::default();
        for byte in bytes {
            decoder.push(std::slice::from_ref(byte), &mut text);
        }
        assert_eq!(text, "ok é\u{2713}");

        let mut text = String::new();
        decoder.push(&[0xff, b'a'], &mut text);
        assert_eq!(text, "\u{fffd}a");
    }

    #[test]
    fn extracts_last_usb_tree_block() {
        let text = "USB device tree:\n  1 Hub\n  +-2 Mass Storage Kingston\n=> ";
        assert_eq!(
            extract_last_usb_tree_summary(text),
            "1 Hub\n  +-2 Mass Storage Kingston"
        );
    }
}
