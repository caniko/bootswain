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
    text.starts_with("=> ")
        || text.contains("\n=> ")
        || text.ends_with("=> ")
        || text.ends_with("=>")
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
        classify_autoboot, extract_last_usb_tree_summary, segment_has_prompt, segment_has_reset,
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
    fn extracts_last_usb_tree_block() {
        let text = "USB device tree:\n  1 Hub\n  +-2 Mass Storage Kingston\n=> ";
        assert_eq!(
            extract_last_usb_tree_summary(text),
            "1 Hub\n  +-2 Mass Storage Kingston"
        );
    }
}
