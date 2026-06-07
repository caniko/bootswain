use super::{
    DEFAULT_QEMU_ARM64_CPU, QemuArm64RunConfig, QemuCommandLine, QemuProcessOutput,
    QemuProcessRunner, SystemQemuProcessRunner, build_qemu_arm64_command,
    run_qemu_arm64_validation_with_runner,
};
use anyhow::{Result, bail};
use bootswain_core::{
    Board, BootTarget, QemuDiskInterface, ValidationExecutorKind, ValidationOutcomeStatus,
    ValidationPlan, ValidationScenario, ValidationScenarioKind, ValidationStep,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::tempdir;

#[derive(Debug)]
struct FakeRunner {
    output: Result<QemuProcessOutput, String>,
    seen_stdin: Option<String>,
}

impl QemuProcessRunner for FakeRunner {
    fn run_qemu(
        &mut self,
        _command: &QemuCommandLine,
        stdin: &str,
        _timeout: Duration,
    ) -> Result<QemuProcessOutput> {
        self.seen_stdin = Some(stdin.to_owned());
        match &self.output {
            Ok(output) => Ok(output.clone()),
            Err(error) => bail!(error.clone()),
        }
    }
}

#[test]
fn builds_base_qemu_command() {
    let config = sample_config(None, QemuDiskInterface::Virtio);
    let command = build_qemu_arm64_command(&config).expect("command");

    assert_eq!(command.program, PathBuf::from("qemu-system-aarch64"));
    assert_eq!(command.args[0..2], ["-machine", "virt"]);
    assert!(command.args.contains(&"-nographic".to_owned()));
    assert_eq!(
        command
            .args
            .windows(2)
            .find(|window| window[0] == "-cpu")
            .expect("cpu arg"),
        ["-cpu", "cortex-a57"]
    );
    let cpu_position = command
        .args
        .iter()
        .position(|arg| arg == "-cpu")
        .expect("cpu position");
    let bios_position = command
        .args
        .iter()
        .position(|arg| arg == "-bios")
        .expect("bios position");
    assert!(cpu_position < bios_position);
    assert!(command.args.contains(&"-bios".to_owned()));
    assert!(command.args.contains(&"u-boot.bin".to_owned()));
}

#[test]
fn builds_disk_interfaces() {
    let cases = [
        (
            QemuDiskInterface::Virtio,
            "virtio-blk-device,drive=bootswain0",
        ),
        (
            QemuDiskInterface::UsbStorage,
            "usb-storage,drive=bootswain0,bus=xhci.0",
        ),
        (
            QemuDiskInterface::Nvme,
            "nvme,serial=bootswain,drive=bootswain0",
        ),
    ];

    for (interface, device_arg) in cases {
        let command =
            build_qemu_arm64_command(&sample_config(Some("disk.img"), interface)).expect("command");
        assert!(
            command
                .args
                .iter()
                .any(|arg| arg == "if=none,file=disk.img,format=raw,id=bootswain0")
        );
        assert!(command.args.iter().any(|arg| arg == device_arg));
    }
}

#[test]
fn rejects_comma_in_disk_path() {
    let error = build_qemu_arm64_command(&sample_config(
        Some("bad,disk.img"),
        QemuDiskInterface::Virtio,
    ))
    .expect_err("comma path should fail");

    assert!(error.to_string().contains("contains a comma"));
}

#[test]
fn fake_runner_success_marks_matching_scenario_passed() {
    let tmp = tempdir().expect("tempdir");
    let u_boot = tmp.path().join("u-boot.bin");
    fs::write(&u_boot, b"fake").expect("write u-boot");
    let config = QemuArm64RunConfig {
        u_boot,
        out: tmp.path().join("out"),
        ..sample_config(None, QemuDiskInterface::Virtio)
    };
    let mut runner = FakeRunner {
        output: Ok(QemuProcessOutput {
            stdout: b"U-Boot 2026.04\n=> ".to_vec(),
            stderr: Vec::new(),
            status_code: None,
            timed_out: true,
        }),
        seen_stdin: None,
    };

    let run = run_qemu_arm64_validation_with_runner(sample_plan(), &config, &mut runner)
        .expect("run qemu validation");

    assert!(run.passed());
    assert_eq!(run.outcomes[0].status, ValidationOutcomeStatus::Passed);
    assert_eq!(runner.seen_stdin, Some("bootflow scan\n".to_owned()));
    assert!(config.out.join("validation-run.json").exists());
    assert!(config.out.join("serial.log").exists());
    assert!(config.out.join("qemu-command.json").exists());
}

#[test]
fn fake_runner_missing_pattern_marks_failure() {
    let tmp = tempdir().expect("tempdir");
    let u_boot = tmp.path().join("u-boot.bin");
    fs::write(&u_boot, b"fake").expect("write u-boot");
    let config = QemuArm64RunConfig {
        u_boot,
        out: tmp.path().join("out"),
        ..sample_config(None, QemuDiskInterface::Virtio)
    };
    let mut runner = FakeRunner {
        output: Ok(QemuProcessOutput {
            stdout: b"no prompt yet".to_vec(),
            stderr: Vec::new(),
            status_code: None,
            timed_out: true,
        }),
        seen_stdin: None,
    };

    let run = run_qemu_arm64_validation_with_runner(sample_plan(), &config, &mut runner)
        .expect("run qemu validation");

    assert_eq!(run.outcomes[0].status, ValidationOutcomeStatus::Failed);
    assert!(
        run.outcomes[0]
            .failure
            .as_deref()
            .expect("failure")
            .contains("missing expected serial patterns")
    );
}

#[test]
fn dry_run_materializes_not_run_without_runner() {
    let tmp = tempdir().expect("tempdir");
    let u_boot = tmp.path().join("u-boot.bin");
    fs::write(&u_boot, b"fake").expect("write u-boot");
    let config = QemuArm64RunConfig {
        u_boot,
        out: tmp.path().join("out"),
        dry_run: true,
        ..sample_config(None, QemuDiskInterface::Virtio)
    };
    let mut runner = FakeRunner {
        output: Err("runner should not be called".to_owned()),
        seen_stdin: None,
    };

    let run = run_qemu_arm64_validation_with_runner(sample_plan(), &config, &mut runner)
        .expect("dry run");

    assert_eq!(run.outcomes[0].status, ValidationOutcomeStatus::NotRun);
    assert!(runner.seen_stdin.is_none());
    assert!(config.out.join("qemu-command.json").exists());
}

#[test]
fn rockpro64_plan_is_rejected_by_qemu_executor() {
    let tmp = tempdir().expect("tempdir");
    let u_boot = tmp.path().join("u-boot.bin");
    fs::write(&u_boot, b"fake").expect("write u-boot");
    let config = QemuArm64RunConfig {
        u_boot,
        out: tmp.path().join("out"),
        ..sample_config(None, QemuDiskInterface::Virtio)
    };
    let mut plan = sample_plan();
    plan.board = Board::RockPro64;
    let mut runner = FakeRunner {
        output: Ok(QemuProcessOutput {
            stdout: Vec::new(),
            stderr: Vec::new(),
            status_code: Some(0),
            timed_out: false,
        }),
        seen_stdin: None,
    };

    let error = run_qemu_arm64_validation_with_runner(plan, &config, &mut runner)
        .expect_err("rockpro64 qemu execution should fail");

    assert!(
        error
            .to_string()
            .contains("ROCKPro64 claims require hardware")
    );
}

#[test]
fn system_runner_times_out_and_kills_process() {
    let mut runner = SystemQemuProcessRunner;
    let command = QemuCommandLine {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), "sleep 2".into()],
    };

    let output = runner
        .run_qemu(&command, "", Duration::from_millis(20))
        .expect("timeout run");

    assert!(output.timed_out);
}

#[test]
fn optional_qemu_smoke_runs_when_requested() {
    if std::env::var("BOOTSWAIN_QEMU_SMOKE").ok().as_deref() != Some("1") {
        return;
    }
    let u_boot = match std::env::var("BOOTSWAIN_QEMU_UBOOT") {
        Ok(path) => PathBuf::from(path),
        Err(_) => return,
    };
    let tmp = tempdir().expect("tempdir");
    let config = QemuArm64RunConfig {
        qemu_binary: PathBuf::from("qemu-system-aarch64"),
        u_boot,
        cpu: DEFAULT_QEMU_ARM64_CPU.to_owned(),
        disk: None,
        disk_interface: QemuDiskInterface::Virtio,
        timeout: Duration::from_secs(5),
        out: tmp.path().join("out"),
        dry_run: false,
    };
    let mut runner = SystemQemuProcessRunner;

    let run = run_qemu_arm64_validation_with_runner(sample_plan(), &config, &mut runner)
        .expect("qemu smoke");

    assert!(run.outcomes.iter().any(|outcome| {
        matches!(
            outcome.status,
            ValidationOutcomeStatus::Passed | ValidationOutcomeStatus::Failed
        )
    }));
}

fn sample_config(disk: Option<&str>, disk_interface: QemuDiskInterface) -> QemuArm64RunConfig {
    QemuArm64RunConfig {
        qemu_binary: PathBuf::from("qemu-system-aarch64"),
        u_boot: PathBuf::from("u-boot.bin"),
        cpu: DEFAULT_QEMU_ARM64_CPU.to_owned(),
        disk: disk.map(PathBuf::from),
        disk_interface,
        timeout: Duration::from_secs(1),
        out: PathBuf::from("out"),
        dry_run: false,
    }
}

fn sample_plan() -> ValidationPlan {
    ValidationPlan {
        schema_version: 1,
        board: Board::GenericArm64Qemu,
        release: "qemu-smoke".into(),
        executor: ValidationExecutorKind::QemuArm64,
        qemu: None,
        scenarios: vec![ValidationScenario {
            name: "prompt".into(),
            kind: ValidationScenarioKind::Prompt,
            target: Some(BootTarget::Virtio),
            protocol: None,
            artifact: None,
            steps: vec![ValidationStep {
                name: "prompt".into(),
                command: Some("bootflow scan".into()),
                expect: vec!["=> ".into()],
                timeout_secs: 5,
            }],
        }],
    }
}
