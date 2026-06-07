use anyhow::{Context, Result, bail};
use bootswain_core::{
    Board, QemuDiskInterface, ValidationExecutorKind, ValidationOutcome, ValidationOutcomeStatus,
    ValidationPlan, ValidationRun, ValidationScenario, ValidationScenarioKind,
    write_pretty_json_file,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_QEMU_ARM64_CPU: &str = "cortex-a57";

#[derive(Debug, Clone)]
pub struct QemuArm64RunConfig {
    pub qemu_binary: PathBuf,
    pub u_boot: PathBuf,
    pub cpu: String,
    pub disk: Option<PathBuf>,
    pub disk_interface: QemuDiskInterface,
    pub timeout: Duration,
    pub out: PathBuf,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QemuCommandLine {
    pub program: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QemuProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status_code: Option<i32>,
    pub timed_out: bool,
}

pub trait QemuProcessRunner {
    fn run_qemu(
        &mut self,
        command: &QemuCommandLine,
        stdin: &str,
        timeout: Duration,
    ) -> Result<QemuProcessOutput>;
}

#[derive(Debug, Default)]
pub struct SystemQemuProcessRunner;

impl QemuProcessRunner for SystemQemuProcessRunner {
    fn run_qemu(
        &mut self,
        command: &QemuCommandLine,
        stdin: &str,
        timeout: Duration,
    ) -> Result<QemuProcessOutput> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn {}", command.program.display()))?;

        if let Some(mut child_stdin) = child.stdin.take() {
            let input = stdin.to_owned();
            thread::spawn(move || {
                let _ = child_stdin.write_all(input.as_bytes());
            });
        }

        let mut stdout = child
            .stdout
            .take()
            .context("failed to capture QEMU stdout")?;
        let stdout_thread = thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = stdout.read_to_end(&mut buffer);
            buffer
        });

        let mut stderr = child
            .stderr
            .take()
            .context("failed to capture QEMU stderr")?;
        let stderr_thread = thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = stderr.read_to_end(&mut buffer);
            buffer
        });

        let start = Instant::now();
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait().context("failed to poll QEMU process")? {
                break status;
            }

            if start.elapsed() >= timeout {
                timed_out = true;
                child.kill().context("failed to kill timed-out QEMU")?;
                break child.wait().context("failed to reap timed-out QEMU")?;
            }

            thread::sleep(Duration::from_millis(20));
        };

        let stdout = stdout_thread
            .join()
            .map_err(|_| anyhow::anyhow!("QEMU stdout reader panicked"))?;
        let stderr = stderr_thread
            .join()
            .map_err(|_| anyhow::anyhow!("QEMU stderr reader panicked"))?;

        Ok(QemuProcessOutput {
            stdout,
            stderr,
            status_code: status.code(),
            timed_out,
        })
    }
}

pub fn build_qemu_arm64_command(config: &QemuArm64RunConfig) -> Result<QemuCommandLine> {
    let u_boot = path_arg(&config.u_boot, "U-Boot binary")?;
    let mut args = vec![
        "-machine".to_owned(),
        "virt".to_owned(),
        "-m".to_owned(),
        "1024".to_owned(),
        "-nographic".to_owned(),
        "-monitor".to_owned(),
        "none".to_owned(),
        "-cpu".to_owned(),
        config.cpu.clone(),
        "-bios".to_owned(),
        u_boot,
    ];

    if let Some(disk) = &config.disk {
        let disk = qemu_option_path(disk, "disk image")?;
        args.push("-drive".to_owned());
        args.push(format!("if=none,file={disk},format=raw,id=bootswain0"));
        match config.disk_interface {
            QemuDiskInterface::Virtio => {
                args.push("-device".to_owned());
                args.push("virtio-blk-device,drive=bootswain0".to_owned());
            }
            QemuDiskInterface::UsbStorage => {
                args.push("-device".to_owned());
                args.push("qemu-xhci,id=xhci".to_owned());
                args.push("-device".to_owned());
                args.push("usb-storage,drive=bootswain0,bus=xhci.0".to_owned());
            }
            QemuDiskInterface::Nvme => {
                args.push("-device".to_owned());
                args.push("nvme,serial=bootswain,drive=bootswain0".to_owned());
            }
        }
    }

    Ok(QemuCommandLine {
        program: config.qemu_binary.clone(),
        args,
    })
}

pub fn run_qemu_arm64_validation(
    plan: ValidationPlan,
    config: &QemuArm64RunConfig,
) -> Result<ValidationRun> {
    let mut runner = SystemQemuProcessRunner;
    run_qemu_arm64_validation_with_runner(plan, config, &mut runner)
}

pub fn run_qemu_arm64_validation_with_runner<R>(
    plan: ValidationPlan,
    config: &QemuArm64RunConfig,
    runner: &mut R,
) -> Result<ValidationRun>
where
    R: QemuProcessRunner,
{
    if plan.board != Board::GenericArm64Qemu {
        bail!(
            "qemu-arm64 validates generic-arm64-qemu plans only; ROCKPro64 claims require hardware"
        );
    }
    if let Some(qemu) = &plan.qemu {
        if qemu.machine != "virt" {
            bail!(
                "qemu-arm64 supports only QEMU machine virt, plan requested {}",
                qemu.machine
            );
        }
        if qemu.timeout_secs == 0 {
            bail!("plan qemu.timeout_secs must be at least 1");
        }
    }

    if !config.u_boot.is_file() {
        bail!("U-Boot binary does not exist: {}", config.u_boot.display());
    }

    fs::create_dir_all(&config.out)
        .with_context(|| format!("failed to create {}", config.out.display()))?;

    let command = build_qemu_arm64_command(config)?;
    write_pretty_json_file(config.out.join("qemu-command.json"), &command)?;

    let serial_log_path = config.out.join("serial.log");
    let stderr_log_path = config.out.join("qemu-stderr.log");
    let command_input = scenario_stdin(&plan);

    let run = if config.dry_run {
        fs::write(&serial_log_path, b"")
            .with_context(|| format!("failed to write {}", serial_log_path.display()))?;
        fs::write(&stderr_log_path, b"")
            .with_context(|| format!("failed to write {}", stderr_log_path.display()))?;
        ValidationRun {
            outcomes: plan
                .scenarios
                .iter()
                .map(|scenario| ValidationOutcome {
                    scenario: scenario.name.clone(),
                    status: ValidationOutcomeStatus::NotRun,
                    executor: Some(ValidationExecutorKind::QemuArm64),
                    evidence: vec![format!(
                        "dry-run command: {}",
                        command_for_evidence(&command)
                    )],
                    logs: vec![
                        PathBuf::from("qemu-command.json"),
                        PathBuf::from("serial.log"),
                        PathBuf::from("qemu-stderr.log"),
                    ],
                    failure: Some("dry-run; QEMU was not executed".to_owned()),
                })
                .collect(),
            plan,
        }
    } else {
        let output = runner.run_qemu(&command, &command_input, config.timeout)?;
        fs::write(&serial_log_path, &output.stdout)
            .with_context(|| format!("failed to write {}", serial_log_path.display()))?;
        fs::write(&stderr_log_path, &output.stderr)
            .with_context(|| format!("failed to write {}", stderr_log_path.display()))?;

        let serial = String::from_utf8_lossy(&output.stdout);
        ValidationRun {
            outcomes: plan
                .scenarios
                .iter()
                .map(|scenario| {
                    evaluate_qemu_scenario(
                        scenario,
                        &serial,
                        &output,
                        &[
                            PathBuf::from("qemu-command.json"),
                            PathBuf::from("serial.log"),
                            PathBuf::from("qemu-stderr.log"),
                        ],
                    )
                })
                .collect(),
            plan,
        }
    };

    write_pretty_json_file(config.out.join("validation-run.json"), &run)?;
    Ok(run)
}

fn evaluate_qemu_scenario(
    scenario: &ValidationScenario,
    serial: &str,
    output: &QemuProcessOutput,
    logs: &[PathBuf],
) -> ValidationOutcome {
    if is_hardware_only_scenario(scenario) {
        return ValidationOutcome {
            scenario: scenario.name.clone(),
            status: ValidationOutcomeStatus::Unsupported,
            executor: Some(ValidationExecutorKind::QemuArm64),
            evidence: Vec::new(),
            logs: logs.to_vec(),
            failure: Some("generic ARM64 QEMU cannot validate ROCKPro64 hardware behavior".into()),
        };
    }

    let expected = expected_patterns(scenario);
    if expected.is_empty() {
        return ValidationOutcome {
            scenario: scenario.name.clone(),
            status: ValidationOutcomeStatus::Skipped,
            executor: Some(ValidationExecutorKind::QemuArm64),
            evidence: Vec::new(),
            logs: logs.to_vec(),
            failure: Some("scenario has no expected serial patterns".into()),
        };
    }

    let missing: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|pattern| !serial.contains(pattern))
        .collect();

    if missing.is_empty() {
        let mut evidence = expected
            .iter()
            .copied()
            .map(|pattern| format!("matched serial pattern: {pattern:?}"))
            .collect::<Vec<_>>();
        if output.timed_out {
            evidence.push("QEMU was stopped after timeout once serial was captured".into());
        }
        return ValidationOutcome {
            scenario: scenario.name.clone(),
            status: ValidationOutcomeStatus::Passed,
            executor: Some(ValidationExecutorKind::QemuArm64),
            evidence,
            logs: logs.to_vec(),
            failure: None,
        };
    }

    let timeout = if output.timed_out {
        " after timeout"
    } else {
        ""
    };
    ValidationOutcome {
        scenario: scenario.name.clone(),
        status: ValidationOutcomeStatus::Failed,
        executor: Some(ValidationExecutorKind::QemuArm64),
        evidence: expected
            .iter()
            .copied()
            .filter(|pattern| serial.contains(pattern))
            .map(|pattern| format!("matched serial pattern: {pattern:?}"))
            .collect(),
        logs: logs.to_vec(),
        failure: Some(format!(
            "missing expected serial patterns{timeout}: {}",
            missing
                .iter()
                .map(|pattern| format!("{pattern:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn is_hardware_only_scenario(scenario: &ValidationScenario) -> bool {
    matches!(
        scenario.kind,
        ValidationScenarioKind::SpiInstall | ValidationScenarioKind::SpiErase
    )
}

fn expected_patterns(scenario: &ValidationScenario) -> Vec<&str> {
    scenario
        .steps
        .iter()
        .flat_map(|step| step.expect.iter().map(String::as_str))
        .collect()
}

fn scenario_stdin(plan: &ValidationPlan) -> String {
    let mut input = String::new();
    for scenario in &plan.scenarios {
        for step in &scenario.steps {
            if let Some(command) = &step.command {
                input.push_str(command);
                input.push('\n');
            }
        }
    }
    input
}

fn command_for_evidence(command: &QemuCommandLine) -> String {
    let mut output = command.program.display().to_string();
    for arg in &command.args {
        output.push(' ');
        output.push_str(arg);
    }
    output
}

fn path_arg(path: &Path, label: &str) -> Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .with_context(|| format!("{label} path is not valid UTF-8: {}", path.display()))
}

fn qemu_option_path(path: &Path, label: &str) -> Result<String> {
    let value = path_arg(path, label)?;
    if value.contains(',') {
        bail!("{label} path contains a comma, which is unsafe in QEMU option syntax");
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
