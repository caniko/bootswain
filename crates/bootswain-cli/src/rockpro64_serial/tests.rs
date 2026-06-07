
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
    let mut serial = FakeSerial::with_chunks(&["=> ", "Flashing SPI payload from mmc 0:1\n=> "]);

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

    let error = run_rockpro64_serial_validation_with_session(sample_plan(), &config, &mut serial)
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
