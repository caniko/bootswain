use bootswain_core::{CompressionKind, ImageInfo, ProbeStepResult};
use bootswain_probe::{
    ProbeConfig, SerialIo, ValidationReport, ValidationStatus, load_validation_plan_json,
    load_validation_report_json, rockpro64_usb_validation_plan, run_trial_with_session,
    write_validation_plan_json, write_validation_report_json,
};
use std::collections::VecDeque;
use std::io;
use tempfile::tempdir;

#[derive(Debug)]
struct FakeSerial {
    reads: VecDeque<Vec<u8>>,
    writes: Vec<String>,
}

impl FakeSerial {
    fn new(chunks: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            reads: chunks
                .into_iter()
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
        let Some(chunk) = self.reads.pop_front() else {
            return Ok(0);
        };
        buffer[..chunk.len()].copy_from_slice(&chunk);
        Ok(chunk.len())
    }

    fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.writes
            .push(String::from_utf8_lossy(buffer).into_owned());
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn test_config(out_dir: std::path::PathBuf) -> ProbeConfig {
    let mut config = ProbeConfig::new(
        ImageInfo {
            path: "/tmp/test.img".into(),
            compression: CompressionKind::None,
            size_bytes: 123,
            sha256: "deadbeef".into(),
        },
        "/dev/ttyUSB0".into(),
        115200,
        out_dir,
        1,
    );
    config.prompt_timeout = std::time::Duration::from_millis(10);
    config.command_timeout = std::time::Duration::from_millis(10);
    config.idle_sleep = std::time::Duration::from_millis(0);
    config
}

#[test]
fn validation_plan_roundtrips_with_fake_serial() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let plan = rockpro64_usb_validation_plan(ImageInfo {
        path: "/tmp/test.img".into(),
        compression: CompressionKind::None,
        size_bytes: 123,
        sha256: "deadbeef".into(),
    });
    let scenario = plan.scenario("clean-usb-sequence").expect("clean scenario");

    let mut serial = FakeSerial::new([
        "U-Boot 2026.04\nHit any key to stop autoboot: 0\n=> ",
        "starting USB...\nBus usb@fe900000: 2 USB Device(s) found\n=> ",
        "USB device tree:\n  1 Hub\n=> ",
        "resetting USB...\nBus usb@fe900000: 2 USB Device(s) found\n=> ",
        "USB device tree:\n  1 Hub\n  +-2  Mass Storage Kingston\n=> ",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("clean trial");

    assert_eq!(trial.usb_start_result, ProbeStepResult::Ok);
    let outcome = scenario.evaluate(&trial);
    assert_eq!(outcome.status, ValidationStatus::Passed);

    let report = bootswain_probe::ValidationReport::new(plan.clone(), vec![outcome.clone()]);
    assert!(report.is_passing());

    let plan_path = tmp.path().join("validation-plan.json");
    let report_path = tmp.path().join("validation-report.json");
    write_validation_plan_json(&plan_path, &plan).expect("write plan");
    write_validation_report_json(&report_path, &report).expect("write report");

    let loaded_plan = load_validation_plan_json(&plan_path).expect("load plan");
    let loaded_report = load_validation_report_json(&report_path).expect("load report");
    assert_eq!(loaded_plan, plan);
    assert_eq!(loaded_report, report);
    assert_eq!(
        serial.writes,
        vec!["usb start\n", "usb tree\n", "usb reset\n", "usb tree\n"]
    );
}

#[test]
fn validation_scenario_reports_mismatch_for_reset_during_usb_start() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let plan = rockpro64_usb_validation_plan(ImageInfo {
        path: "/tmp/test.img".into(),
        compression: CompressionKind::None,
        size_bytes: 123,
        sha256: "deadbeef".into(),
    });
    let scenario = plan.scenario("clean-usb-sequence").expect("clean scenario");

    let mut serial = FakeSerial::new([
        "U-Boot 2026.04\n=> ",
        "starting USB...\nResetting CPU ...\n",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("reset trial");

    let outcome = scenario.evaluate(&trial);
    assert_eq!(outcome.status, ValidationStatus::Failed);
    assert!(
        outcome
            .mismatches
            .iter()
            .any(|mismatch| mismatch.contains("usb_start_result"))
    );

    let report = ValidationReport::new(plan, vec![outcome]);
    assert!(!report.is_passing());
}

#[test]
fn usb_start_reset_scenario_passes_for_matching_failure() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let plan = rockpro64_usb_validation_plan(ImageInfo {
        path: "/tmp/test.img".into(),
        compression: CompressionKind::None,
        size_bytes: 123,
        sha256: "deadbeef".into(),
    });
    let scenario = plan.scenario("usb-start-reset").expect("reset scenario");

    let mut serial = FakeSerial::new([
        "U-Boot 2026.04\n=> ",
        "starting USB...\nResetting CPU ...\n",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("reset trial");

    let outcome = scenario.evaluate(&trial);
    assert_eq!(outcome.status, ValidationStatus::Passed);
    assert!(outcome.mismatches.is_empty());
}

#[test]
fn validation_plan_loader_rejects_unknown_board() {
    let tmp = tempdir().expect("tempdir");
    let plan_path = tmp.path().join("validation-plan.json");
    std::fs::write(
        &plan_path,
        r#"{
  "name": "bad-plan",
  "board": "rockpro64-v2",
  "image": {
    "path": "/tmp/test.img",
    "compression": "none",
    "size_bytes": 123,
    "sha256": "deadbeef"
  },
  "scenarios": []
}"#,
    )
    .expect("write bad plan");

    let error = load_validation_plan_json(&plan_path).expect_err("bad board rejected");
    assert!(error.to_string().contains("unknown board"));
}

#[test]
fn default_usb_validation_plan_covers_expected_failure_modes() {
    let plan = rockpro64_usb_validation_plan(ImageInfo {
        path: "/tmp/test.img".into(),
        compression: CompressionKind::None,
        size_bytes: 123,
        sha256: "deadbeef".into(),
    });

    for scenario in [
        "clean-usb-sequence",
        "autoboot-reset",
        "usb-start-reset",
        "usb-start-timeout",
        "usb-tree-reset",
        "usb-reset-reset",
    ] {
        assert!(plan.scenario(scenario).is_some(), "missing {scenario}");
    }
    assert_eq!(plan.scenarios.len(), 6);
}
