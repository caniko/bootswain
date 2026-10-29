use bootswain_core::{AutobootResult, FailureStage, ImageInfo, ProbeStepResult};
use bootswain_probe::{ProbeConfig, SerialIo, run_trial_with_session};
use std::collections::VecDeque;
use std::io;
use tempfile::tempdir;

#[derive(Debug)]
struct MockSerial {
    reads: VecDeque<Vec<u8>>,
    writes: Vec<String>,
}

impl MockSerial {
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

impl SerialIo for MockSerial {
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
            compression: bootswain_core::CompressionKind::None,
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
fn clean_trial_runs_full_sequence() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial = MockSerial::new([
        "U-Boot 2026.04\nHit any key to stop autoboot: 0\n=> ",
        "starting USB...\nBus usb@fe900000: 2 USB Device(s) found\n=> ",
        "USB device tree:\n  1 Hub\n=> ",
        "resetting USB...\nBus usb@fe900000: 2 USB Device(s) found\n=> ",
        "USB device tree:\n  1 Hub\n  +-2  Mass Storage Kingston\n=> ",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("clean trial");

    assert_eq!(trial.autoboot_result, AutobootResult::Other);
    assert_eq!(trial.usb_start_result, ProbeStepResult::Ok);
    assert_eq!(trial.usb_tree_result, ProbeStepResult::Ok);
    assert_eq!(trial.usb_reset_result, ProbeStepResult::Ok);
    assert_eq!(trial.failure_stage, FailureStage::None);
    assert!(trial.final_detected_usb_summary.contains("Kingston"));
    assert_eq!(
        serial.writes,
        vec!["usb start\n", "usb tree\n", "usb reset\n", "usb tree\n"]
    );
}

#[test]
fn reset_during_autoboot_marks_trial_as_failed() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial =
        MockSerial::new(["\"Synchronous Abort\" handler, esr 0x96000010\nResetting CPU ...\n"]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("autoboot reset trial");

    assert_eq!(trial.autoboot_result, AutobootResult::Abort);
    assert_eq!(trial.failure_stage, FailureStage::Autoboot);
    assert_eq!(trial.usb_start_result, ProbeStepResult::NotRun);
    assert_eq!(trial.usb_tree_result, ProbeStepResult::NotRun);
    assert_eq!(trial.usb_reset_result, ProbeStepResult::NotRun);
}

#[test]
fn reset_during_usb_start_is_reported() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial = MockSerial::new([
        "U-Boot 2026.04\n=> ",
        "starting USB...\nResetting CPU ...\n",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("usb start reset trial");

    assert_eq!(trial.usb_start_result, ProbeStepResult::Reset);
    assert_eq!(trial.failure_stage, FailureStage::UsbStart);
    assert_eq!(trial.usb_tree_result, ProbeStepResult::NotRun);
    assert_eq!(trial.usb_reset_result, ProbeStepResult::NotRun);
}

#[test]
fn timeout_during_usb_start_is_reported() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial = MockSerial::new(["U-Boot 2026.04\n=> ", "starting USB...\n"]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("usb start timeout trial");

    assert_eq!(trial.usb_start_result, ProbeStepResult::Timeout);
    assert_eq!(trial.failure_stage, FailureStage::UsbStart);
}

#[test]
fn reset_during_usb_tree_is_reported() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial = MockSerial::new([
        "U-Boot 2026.04\n=> ",
        "starting USB...\n=> ",
        "USB device tree:\nResetting CPU ...\n",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("usb tree reset trial");

    assert_eq!(trial.usb_start_result, ProbeStepResult::Ok);
    assert_eq!(trial.usb_tree_result, ProbeStepResult::Reset);
    assert_eq!(trial.failure_stage, FailureStage::UsbTree1);
    assert_eq!(trial.usb_reset_result, ProbeStepResult::NotRun);
}

#[test]
fn reset_during_usb_reset_is_reported() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial = MockSerial::new([
        "U-Boot 2026.04\n=> ",
        "starting USB...\n=> ",
        "USB device tree:\n  1 Hub\n=> ",
        "resetting USB...\nResetting CPU ...\n",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("usb reset failure");

    assert_eq!(trial.usb_start_result, ProbeStepResult::Ok);
    assert_eq!(trial.usb_tree_result, ProbeStepResult::Ok);
    assert_eq!(trial.usb_reset_result, ProbeStepResult::Reset);
    assert_eq!(trial.failure_stage, FailureStage::UsbReset);
}

#[test]
fn usb_stopped_prompt_state_succeeds_once_usb_start_runs() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial = MockSerial::new([
        "USB is stopped. Please issue 'usb start' first.\n=> ",
        "starting USB...\nBus usb@fe900000: 2 USB Device(s) found\n=> ",
        "USB device tree:\n  1 Hub\n=> ",
        "resetting USB...\nBus usb@fe900000: 2 USB Device(s) found\n=> ",
        "USB device tree:\n  1 Hub\n  +-2  Mass Storage Kingston\n=> ",
    ]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("usb stopped trial");

    assert_eq!(trial.failure_stage, FailureStage::None);
    assert_eq!(trial.usb_start_result, ProbeStepResult::Ok);
    assert!(trial.final_detected_usb_summary.contains("Kingston"));
}

#[test]
fn no_prompt_timeout_marks_autoboot_failure() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let mut serial =
        MockSerial::new(["starting USB...\nBus usb@fe900000: 2 USB Device(s) found\n"]);
    let trial = run_trial_with_session(&mut serial, &test_config(tmp.path().into()), 1, &trial_dir)
        .expect("timeout trial");

    assert_eq!(trial.autoboot_result, AutobootResult::Other);
    assert_eq!(trial.failure_stage, FailureStage::Autoboot);
    assert_eq!(trial.usb_start_result, ProbeStepResult::NotRun);
    assert_eq!(trial.usb_tree_result, ProbeStepResult::NotRun);
    assert_eq!(trial.usb_reset_result, ProbeStepResult::NotRun);
}

/// Never reaches a prompt and never goes quiet.
struct ChattySerial;

impl SerialIo for ChattySerial {
    fn clear(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        buffer[..4].copy_from_slice(b"spam");
        Ok(4)
    }

    fn write_all(&mut self, _buffer: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn continuous_output_still_times_out() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let started = std::time::Instant::now();
    let trial = run_trial_with_session(
        &mut ChattySerial,
        &test_config(tmp.path().into()),
        1,
        &trial_dir,
    )
    .expect("chatty trial");

    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    assert_eq!(trial.failure_stage, FailureStage::Autoboot);
}

/// Yields one chunk of output, then fails.
struct FailingSerial {
    sent: bool,
}

impl SerialIo for FailingSerial {
    fn clear(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.sent {
            return Err(io::Error::other("cable pulled"));
        }
        self.sent = true;
        buffer[..11].copy_from_slice(b"U-Boot 2026");
        Ok(11)
    }

    fn write_all(&mut self, _buffer: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn serial_error_still_persists_partial_trial() {
    let tmp = tempdir().expect("tempdir");
    let trial_dir = tmp.path().join("trial-1");
    std::fs::create_dir_all(&trial_dir).expect("trial dir");

    let error = run_trial_with_session(
        &mut FailingSerial { sent: false },
        &test_config(tmp.path().into()),
        1,
        &trial_dir,
    )
    .expect_err("serial failure");

    assert!(format!("{error:#}").contains("cable pulled"));
    let log = std::fs::read_to_string(trial_dir.join("serial.log")).expect("serial log");
    assert_eq!(log, "U-Boot 2026");
    let trial = std::fs::read_to_string(trial_dir.join("trial.json")).expect("trial json");
    assert!(trial.contains("\"failure_stage\": \"autoboot\""));
}
