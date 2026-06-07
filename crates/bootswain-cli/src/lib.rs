use anyhow::Result;
use bootswain_core::{
    FirmwareManifest, FlashExecutionResult, FlashPlan, FlashVerificationResult, ImageInfo,
    SummaryResult, ValidationRun,
};
use std::fmt::Write as _;

pub mod qemu;
pub mod rockpro64_serial;

pub fn render_image_info_human(info: &ImageInfo) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "path: {}", info.path.display());
    let _ = writeln!(output, "compression: {}", info.compression);
    let _ = writeln!(output, "size-bytes: {}", info.size_bytes);
    let _ = writeln!(output, "sha256: {}", info.sha256);
    output
}

pub fn render_flash_plan_human(plan: &FlashPlan) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "image: {}", plan.image.path.display());
    let _ = writeln!(output, "device: {}", plan.device.canonical_path.display());
    let _ = writeln!(output, "block-name: {}", plan.device.block_name);
    let _ = writeln!(output, "device-kind: {}", plan.device.kind);
    let _ = writeln!(output, "compression: {}", plan.image.compression);
    let _ = writeln!(output, "image-size-bytes: {}", plan.image.size_bytes);
    let _ = writeln!(output, "sha256: {}", plan.image.sha256);
    let _ = writeln!(output, "strategy: {}", plan.strategy);
    let _ = writeln!(
        output,
        "dry-run: {}",
        if plan.dry_run { "yes" } else { "no" }
    );
    let _ = writeln!(
        output,
        "device-size-bytes: {}",
        plan.device
            .size_bytes
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string())
    );
    let _ = writeln!(
        output,
        "removable: {}",
        plan.device.removable.map_or_else(
            || "unknown".to_owned(),
            |value| if value {
                "yes".to_owned()
            } else {
                "no".to_owned()
            }
        )
    );
    if let Some(vendor) = &plan.device.vendor {
        let _ = writeln!(output, "vendor: {vendor}");
    }
    if let Some(model) = &plan.device.model {
        let _ = writeln!(output, "model: {model}");
    }
    output
}

pub fn render_flash_verification_human(verification: &FlashVerificationResult) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "verification: {}",
        if verification.matched { "ok" } else { "failed" }
    );
    let _ = writeln!(output, "verified-bytes: {}", verification.verified_bytes);
    let _ = writeln!(
        output,
        "verified-image: {}",
        verification.image_path.display()
    );
    let _ = writeln!(
        output,
        "verified-device: {}",
        verification.device_path.display()
    );
    output
}

pub fn render_firmware_manifest_human(manifest: &FirmwareManifest) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "release: {}", manifest.release);
    let _ = writeln!(output, "board: {}", manifest.board);
    let _ = writeln!(output, "schema-version: {}", manifest.schema_version);
    let _ = writeln!(output, "u-boot: {}", manifest.sources.u_boot);
    let _ = writeln!(
        output,
        "trusted-firmware-a: {}",
        manifest
            .sources
            .trusted_firmware_a
            .as_deref()
            .unwrap_or("not-applicable")
    );
    let _ = writeln!(
        output,
        "spi-size-bytes: {}",
        manifest
            .storage_layout
            .spi_size_bytes
            .map_or_else(|| "not-applicable".to_owned(), |value| value.to_string())
    );
    let _ = writeln!(
        output,
        "persistent-environment: {}",
        if manifest.environment_policy.persistent {
            "yes"
        } else {
            "no"
        }
    );
    let _ = writeln!(output, "artifacts: {}", manifest.artifacts.len());
    let _ = writeln!(
        output,
        "validation-claims: {}",
        manifest.validation_claims.len()
    );
    if !manifest.unsupported_paths.is_empty() {
        let _ = writeln!(output, "unsupported-paths:");
        for path in &manifest.unsupported_paths {
            let _ = writeln!(output, "- {path}");
        }
    }
    output
}

pub fn render_firmware_artifacts_human(manifest: &FirmwareManifest) -> String {
    let mut output = String::new();
    for artifact in &manifest.artifacts {
        let _ = writeln!(
            output,
            "{}: {} ({} bytes, flashable: {})",
            artifact.kind,
            artifact.path.display(),
            artifact.size_bytes,
            if artifact.flashable { "yes" } else { "no" }
        );
    }
    output
}

pub fn render_probe_summary_human(summary: &SummaryResult) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Probe summary");
    let _ = writeln!(output, "board: {}", summary.board);
    let _ = writeln!(output, "image: {}", summary.image_path.display());
    let _ = writeln!(output, "total-trials: {}", summary.total_trials);
    let _ = writeln!(output, "clean-trials: {}", summary.clean_trials);
    let _ = writeln!(output, "failing-trials: {}", summary.failing_trials);

    for trial in &summary.trial_results {
        let _ = writeln!(
            output,
            "trial {}: {} (failure-stage: {}, autoboot: {}, usb-start: {}, usb-tree: {}, usb-reset: {})",
            trial.trial_number,
            if trial.is_clean() { "clean" } else { "failed" },
            trial.failure_stage,
            match trial.autoboot_result {
                bootswain_core::AutobootResult::Abort => "abort",
                bootswain_core::AutobootResult::WarnEnumerate => "warn+enumerate",
                bootswain_core::AutobootResult::NoDevice => "no-device",
                bootswain_core::AutobootResult::Other => "other",
            },
            trial.usb_start_result,
            trial.usb_tree_result,
            trial.usb_reset_result
        );
    }

    output
}

pub fn render_validation_run_human(run: &ValidationRun) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "Validation run");
    let _ = writeln!(output, "board: {}", run.plan.board);
    let _ = writeln!(output, "release: {}", run.plan.release);
    let _ = writeln!(output, "scenarios: {}", run.plan.scenarios.len());
    for outcome in &run.outcomes {
        let _ = writeln!(output, "{}: {}", outcome.scenario, outcome.status);
        if let Some(failure) = &outcome.failure {
            let _ = writeln!(output, "  failure: {failure}");
        }
    }
    output
}

pub fn run_flash_execution<F>(plan: &FlashPlan, mut executor: F) -> Result<FlashExecutionResult>
where
    F: FnMut(&FlashPlan) -> Result<u64>,
{
    let bytes_written = if plan.dry_run {
        None
    } else {
        Some(executor(plan)?)
    };

    Ok(FlashExecutionResult {
        plan: plan.clone(),
        bytes_written,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        render_flash_plan_human, render_image_info_human, render_probe_summary_human,
        run_flash_execution,
    };
    use bootswain_core::{
        AutobootResult, BlockDeviceInfo, BlockDeviceKind, Board, CompressionKind, FailureStage,
        FlashPlan, FlashStrategy, ImageInfo, ProbeStepResult, SummaryResult, TrialResult,
    };
    use std::path::PathBuf;

    fn sample_image() -> ImageInfo {
        ImageInfo {
            path: PathBuf::from("/tmp/image.img"),
            compression: CompressionKind::None,
            size_bytes: 1024,
            sha256: "deadbeef".into(),
        }
    }

    fn sample_plan(dry_run: bool) -> FlashPlan {
        FlashPlan {
            image: sample_image(),
            device: BlockDeviceInfo {
                original_path: PathBuf::from("/dev/disk/by-id/mock"),
                canonical_path: PathBuf::from("/dev/sdb"),
                block_name: "sdb".into(),
                kind: BlockDeviceKind::Disk,
                size_bytes: Some(2048),
                removable: Some(true),
                model: Some("USB Reader".into()),
                vendor: Some("Kingston".into()),
            },
            strategy: FlashStrategy::RawCopy,
            dry_run,
        }
    }

    #[test]
    fn dry_run_skips_flash_executor() {
        let mut called = false;
        let result = run_flash_execution(&sample_plan(true), |_| {
            called = true;
            Ok(0)
        })
        .expect("dry-run execution");

        assert!(!called);
        assert_eq!(result.bytes_written, None);
    }

    #[test]
    fn renders_human_outputs() {
        let image_text = render_image_info_human(&sample_image());
        assert!(image_text.contains("sha256: deadbeef"));

        let flash_text = render_flash_plan_human(&sample_plan(true));
        assert!(flash_text.contains("device-kind: disk"));
        assert!(flash_text.contains("vendor: Kingston"));

        let summary = SummaryResult {
            image_path: PathBuf::from("/tmp/image.img"),
            image_sha256: "deadbeef".into(),
            board: Board::RockPro64,
            total_trials: 1,
            clean_trials: 0,
            failing_trials: 1,
            trial_results: vec![TrialResult {
                image_path: PathBuf::from("/tmp/image.img"),
                image_sha256: "deadbeef".into(),
                board: Board::RockPro64,
                serial_port: "/dev/ttyUSB0".into(),
                baud: 115200,
                trial_number: 1,
                autoboot_result: AutobootResult::Other,
                usb_start_result: ProbeStepResult::Timeout,
                usb_tree_result: ProbeStepResult::NotRun,
                usb_reset_result: ProbeStepResult::NotRun,
                failure_stage: FailureStage::UsbStart,
                final_detected_usb_summary: String::new(),
                raw_log_path: PathBuf::from("serial.log"),
            }],
        };

        let probe_text = render_probe_summary_human(&summary);
        assert!(probe_text.contains("failure-stage: usb-start"));
    }

    #[test]
    fn flash_plan_json_contains_device_metadata_and_dry_run() {
        let json = serde_json::to_string_pretty(&sample_plan(true)).expect("flash plan json");
        assert!(json.contains("\"dry_run\": true"));
        assert!(json.contains("\"block_name\": \"sdb\""));
    }

    #[test]
    fn probe_json_output_contains_failure_stage() {
        let summary = SummaryResult {
            image_path: PathBuf::from("/tmp/image.img"),
            image_sha256: "deadbeef".into(),
            board: Board::RockPro64,
            total_trials: 1,
            clean_trials: 1,
            failing_trials: 0,
            trial_results: vec![TrialResult {
                image_path: PathBuf::from("/tmp/image.img"),
                image_sha256: "deadbeef".into(),
                board: Board::RockPro64,
                serial_port: "/dev/ttyUSB0".into(),
                baud: 115200,
                trial_number: 1,
                autoboot_result: AutobootResult::Other,
                usb_start_result: ProbeStepResult::Ok,
                usb_tree_result: ProbeStepResult::Ok,
                usb_reset_result: ProbeStepResult::Ok,
                failure_stage: FailureStage::None,
                final_detected_usb_summary: "Kingston".into(),
                raw_log_path: PathBuf::from("serial.log"),
            }],
        };

        let json = serde_json::to_string_pretty(&summary).expect("summary json");
        assert!(json.contains("\"failure_stage\": \"none\""));
        assert!(json.contains("\"usb_start_result\": \"ok\""));
    }
}
