use super::{run_serial_validation_with_session, serial_selected_outcomes_passed};
use bootswain_core::{Board, ValidationExecutorKind, ValidationOutcomeStatus};
use std::sync::atomic::Ordering;
use tempfile::tempdir;

mod support;

use support::{FakeSerial, sample_config, sample_plan};

#[test]
fn prompt_scenario_passes_and_writes_log() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(
        tmp.path().join("out"),
        vec!["recovery-console".into()],
        false,
    );
    let mut serial = FakeSerial::with_chunks(&["U-Boot 2026.04\n=> "]);

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
        .expect("serial validation");

    let outcome = run
        .outcomes
        .iter()
        .find(|outcome| outcome.scenario == "recovery-console")
        .expect("recovery outcome");
    assert_eq!(outcome.status, ValidationOutcomeStatus::Passed);
    assert!(serial_selected_outcomes_passed(&run));
    assert!(config.out.join("recovery-console/serial.log").exists());
}

#[test]
fn command_step_waits_for_prompt_before_write() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
    let mut serial = FakeSerial::with_chunks(&["=> ", "Scanning for bootflows\n=> "]);

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run =
        run_serial_validation_with_session(plan, &config, &mut serial).expect("serial validation");

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
    assert!(!serial_selected_outcomes_passed(&run));
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
        .expect("serial validation");

    let outcome = run
        .outcomes
        .iter()
        .find(|outcome| outcome.scenario == "spi-install")
        .expect("spi outcome");
    assert_eq!(outcome.status, ValidationOutcomeStatus::Skipped);
    assert!(serial.writes.is_empty());
    assert!(!serial_selected_outcomes_passed(&run));
}

#[test]
fn destructive_spi_scenarios_run_with_flag() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(tmp.path().join("out"), vec!["spi-install".into()], true);
    let mut serial = FakeSerial::with_chunks(&[
        "=> ",
        "Flashing SPI payload from mmc 0:1\nSPI flash complete\n",
    ]);

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run =
        run_serial_validation_with_session(plan, &config, &mut serial).expect("serial validation");

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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
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

    let error = run_serial_validation_with_session(sample_plan(), &config, &mut serial)
        .expect_err("missing scenario should fail");

    assert!(error.to_string().contains("does not contain scenario"));
}

struct BrokenSerial;

impl bootswain_probe::SerialIo for BrokenSerial {
    fn clear(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn read_chunk(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("adapter unplugged"))
    }

    fn write_all(&mut self, _buffer: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn serial_io_error_is_recorded_and_run_file_still_written() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(
        tmp.path().join("out"),
        vec!["recovery-console".into()],
        false,
    );

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut BrokenSerial)
        .expect("run is returned with the failure recorded");

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
            .is_some_and(|failure| failure.contains("adapter unplugged"))
    );
    assert!(config.out.join("validation-run.json").exists());
    assert!(config.out.join("recovery-console/serial.log").exists());
    assert!(!serial_selected_outcomes_passed(&run));
}

fn pi3_plan(executor: ValidationExecutorKind, board: Board) -> bootswain_core::ValidationPlan {
    let mut plan = sample_plan();
    plan.board = board;
    plan.executor = executor;
    plan
}

#[test]
fn u_boot_serial_plan_runs_for_raspberry_pi_3_b_plus() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(
        tmp.path().join("out"),
        vec!["recovery-console".into()],
        false,
    );
    let mut serial = FakeSerial::with_chunks(&["U-Boot 2026.04\n=> "]);
    let plan = pi3_plan(
        ValidationExecutorKind::UBootSerial,
        Board::RaspberryPi3BPlus,
    );

    let run = run_serial_validation_with_session(plan, &config, &mut serial).expect("run");

    assert!(serial_selected_outcomes_passed(&run));
    assert!(
        run.outcomes
            .iter()
            .all(|outcome| outcome.executor == Some(ValidationExecutorKind::UBootSerial))
    );
}

#[test]
fn rockpro64_executor_is_rejected_for_other_boards() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(tmp.path().join("out"), Vec::new(), false);
    let mut serial = FakeSerial::with_chunks(&[]);
    let plan = pi3_plan(
        ValidationExecutorKind::RockPro64Serial,
        Board::RaspberryPi3BPlus,
    );

    let error = run_serial_validation_with_session(plan, &config, &mut serial)
        .expect_err("rockpro64 executor on a pi plan");

    assert!(
        error
            .to_string()
            .contains("only valid for the rockpro64 board")
    );
}

#[test]
fn prompt_only_step_requests_the_prompt_through_boot_banners() {
    let tmp = tempdir().expect("tempdir");
    let mut config = sample_config(
        tmp.path().join("out"),
        vec!["recovery-console".into()],
        false,
    );
    config.idle_sleep = std::time::Duration::from_millis(10);
    let mut plan = sample_plan();
    plan.scenarios[0].steps[0].timeout_secs = 3;
    // A normal boot banner (\nU-Boot) must not count as a reset while waiting for a prompt,
    // and silence must trigger the autoboot interrupt.
    let mut serial = FakeSerial::with_chunks(&["\nU-Boot 2026.04\nHit any key to stop autoboot"]);

    let run =
        run_serial_validation_with_session(plan, &config, &mut serial).expect("serial validation");

    let outcome = &run.outcomes[0];
    assert_eq!(outcome.status, ValidationOutcomeStatus::Failed);
    assert!(
        outcome
            .failure
            .as_deref()
            .is_some_and(|failure| failure.contains("timed out")),
        "{:?}",
        outcome.failure
    );
    assert_eq!(serial.writes, vec!["\n"]);
}

#[test]
fn command_step_waits_for_board_specific_prompt() {
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
    let mut serial = FakeSerial::with_chunks(&[
        "U-Boot 2026.04\nHit any key to stop autoboot:  0\nU-Boot> ",
        "Scanning for bootflows\nU-Boot> ",
    ]);
    let mut plan = sample_plan();
    plan.uboot_prompt = Some("U-Boot> ".into());

    let run = run_serial_validation_with_session(plan, &config, &mut serial).expect("run");

    assert!(serial_selected_outcomes_passed(&run));
    assert_eq!(serial.writes, vec!["bootflow scan\n"]);
}

#[test]
fn failed_bootflow_does_not_end_the_scan() {
    // Pi 3 B+ serial log: bootstd tries efi_mgr first, it fails, extlinux then boots.
    let tmp = tempdir().expect("tempdir");
    let config = sample_config(tmp.path().join("out"), vec!["bootflow".into()], false);
    let mut serial = FakeSerial::with_chunks(&[
        "=> ",
        "** Booting bootflow '<NULL>' with efi_mgr\nBoot failed (err=-14)\n",
        "** Booting bootflow 'mmc.part_2' with extlinux\nScanning for bootflows\n",
    ]);

    let run = run_serial_validation_with_session(sample_plan(), &config, &mut serial).expect("run");

    assert!(serial_selected_outcomes_passed(&run));
}
