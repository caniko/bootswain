use anyhow::{Context, Result, bail};
use bootswain_cli::{
    qemu::{DEFAULT_QEMU_ARM64_CPU, QemuArm64RunConfig, run_qemu_arm64_validation},
    render_firmware_artifacts_human, render_firmware_manifest_human, render_flash_plan_human,
    render_flash_verification_human, render_image_info_human, render_probe_summary_human,
    render_validation_run_human,
    rockpro64_serial::{
        RockPro64SerialRunConfig, rockpro64_serial_selected_outcomes_passed,
        run_rockpro64_serial_validation,
    },
    run_flash_execution,
};
use bootswain_core::{
    FirmwareArtifact, FirmwareArtifactKind, FirmwareManifest, FlashExecutionResult, FlashRunResult,
    QemuDiskInterface, ValidationExecutorKind, ValidationOutcome, ValidationOutcomeStatus,
    ValidationPlan, ValidationRun,
};
use bootswain_flash::{
    execute_flash, inspect_image, plan_flash, validate_required_device_size,
    validate_target_device, verify_flash,
};
use bootswain_probe::{ProbeConfig, run_rockpro64_usb_probe};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(name = "bootswain")]
#[command(about = "ROCKPro64-first host-side flash and probe tooling for stock U-Boot")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Image {
        #[command(subcommand)]
        command: ImageCommand,
    },
    Flash {
        #[command(subcommand)]
        command: FlashCommand,
    },
    Firmware {
        #[command(subcommand)]
        command: FirmwareCommand,
    },
    Probe {
        #[command(subcommand)]
        command: ProbeCommand,
    },
    Validate {
        #[command(subcommand)]
        command: ValidateCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ImageCommand {
    Inspect {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum FlashCommand {
    Sd {
        #[arg(long)]
        image: Option<PathBuf>,
        #[arg(long)]
        device: PathBuf,
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        artifact: Option<FirmwareArtifactKind>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        verify: bool,
        #[arg(long)]
        verify_only: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum FirmwareCommand {
    Inspect {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Artifacts {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProbeCommand {
    Rockpro64Usb {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        port: String,
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        #[arg(long, default_value = "bootswain-probe-output")]
        out: PathBuf,
        #[arg(long, default_value_t = 1)]
        repeat: u32,
        #[arg(long, default_value_t = 20)]
        prompt_timeout_secs: u64,
        #[arg(long, default_value_t = 10)]
        command_timeout_secs: u64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ValidateCommand {
    Run {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long, default_value = "bootswain-validation-output")]
        out: PathBuf,
        #[arg(long)]
        json: bool,
    },
    QemuArm64 {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        u_boot: PathBuf,
        #[arg(long, default_value = "qemu-system-aarch64")]
        qemu: PathBuf,
        #[arg(long, default_value = "bootswain-qemu-arm64-output")]
        out: PathBuf,
        #[arg(long)]
        disk: Option<PathBuf>,
        #[arg(long, default_value = "virtio")]
        disk_interface: QemuDiskInterface,
        #[arg(long)]
        timeout_secs: Option<u64>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    Rockpro64Serial {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        port: String,
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        #[arg(long, default_value = "bootswain-rockpro64-serial-output")]
        out: PathBuf,
        #[arg(long = "scenario")]
        scenarios: Vec<String>,
        #[arg(long)]
        allow_destructive_spi: bool,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Image { command } => run_image(command),
        Command::Flash { command } => run_flash(command),
        Command::Firmware { command } => run_firmware(command),
        Command::Probe { command } => run_probe(command),
        Command::Validate { command } => run_validate(command),
    }
}

fn run_image(command: ImageCommand) -> Result<()> {
    match command {
        ImageCommand::Inspect { image, json } => {
            let info = inspect_image(&image)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&info).context("failed to render image JSON")?
                );
            } else {
                print!("{}", render_image_info_human(&info));
            }
        }
    }

    Ok(())
}

fn run_flash(command: FlashCommand) -> Result<()> {
    match command {
        FlashCommand::Sd {
            image,
            device,
            manifest,
            artifact,
            yes,
            dry_run,
            verify,
            verify_only,
            json,
        } => {
            if dry_run && verify {
                bail!(
                    "cannot combine --dry-run with --verify; use --verify-only for read-back verification"
                );
            }

            let manifest_bundle = match manifest {
                Some(path) => {
                    let manifest = load_manifest(&path)?;
                    Some((path, manifest))
                }
                None => None,
            };
            let selected_artifact = match (&manifest_bundle, artifact) {
                (Some((manifest_path, manifest)), Some(kind)) => {
                    Some(resolve_flash_artifact(manifest_path, manifest, kind)?)
                }
                (Some(_), None) => bail!("--manifest requires --artifact for flash commands"),
                (None, Some(_)) => bail!("--artifact requires --manifest"),
                (None, None) => None,
            };
            let selected_image = match (&selected_artifact, image) {
                (Some((path, _)), None) => path.clone(),
                (Some(_), Some(_)) => {
                    bail!("pass either --image or --manifest/--artifact, not both")
                }
                (None, Some(path)) => path,
                (None, None) => {
                    bail!("missing image source: pass --image or --manifest with --artifact")
                }
            };

            let image_info = inspect_image(&selected_image)?;
            if let Some((_, artifact)) = &selected_artifact {
                validate_image_against_artifact(&image_info, artifact)?;
            }
            let validated = validate_target_device(&device)?;
            let plan = plan_flash(&image_info, &validated, dry_run);
            if let Some((_, artifact)) = &selected_artifact {
                let required_bytes = artifact
                    .required_device_size_bytes
                    .unwrap_or(plan.image.size_bytes);
                validate_required_device_size(&plan, required_bytes)?;
            }

            if !json {
                print!("{}", render_flash_plan_human(&plan));
            }

            if !dry_run && !verify_only && !yes {
                confirm_device_write(&plan.device.canonical_path, json)?;
            }

            let execution = if verify_only {
                FlashExecutionResult {
                    plan: plan.clone(),
                    bytes_written: None,
                }
            } else {
                run_flash_execution(&plan, execute_flash)?
            };
            let verification = if verify || verify_only {
                Some(verify_flash(&plan)?)
            } else {
                None
            };
            if json {
                if verification.is_some() {
                    let result = FlashRunResult {
                        execution: execution.clone(),
                        verification: verification.clone(),
                    };
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&result)
                            .context("failed to render flash result JSON")?
                    );
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&plan)
                            .context("failed to render flash JSON")?
                    );
                }
                if let Some(bytes_written) = execution.bytes_written {
                    eprintln!(
                        "wrote {} bytes to {}",
                        bytes_written,
                        execution.plan.device.canonical_path.display()
                    );
                }
            } else {
                if let Some(bytes_written) = execution.bytes_written {
                    println!(
                        "wrote {} bytes to {}",
                        bytes_written,
                        execution.plan.device.canonical_path.display()
                    );
                }
                if let Some(verification) = &verification {
                    print!("{}", render_flash_verification_human(verification));
                }
            }

            if let Some(verification) = verification {
                if !verification.matched {
                    bail!("flash verification failed");
                }
            }
        }
    }

    Ok(())
}

fn run_firmware(command: FirmwareCommand) -> Result<()> {
    match command {
        FirmwareCommand::Inspect { manifest, json } => {
            let manifest = load_manifest(&manifest)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&manifest)
                        .context("failed to render manifest JSON")?
                );
            } else {
                print!("{}", render_firmware_manifest_human(&manifest));
            }
        }
        FirmwareCommand::Artifacts { manifest, json } => {
            let manifest = load_manifest(&manifest)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&manifest.artifacts)
                        .context("failed to render artifacts JSON")?
                );
            } else {
                print!("{}", render_firmware_artifacts_human(&manifest));
            }
        }
    }

    Ok(())
}

fn run_probe(command: ProbeCommand) -> Result<()> {
    match command {
        ProbeCommand::Rockpro64Usb {
            image,
            port,
            baud,
            out,
            repeat,
            prompt_timeout_secs,
            command_timeout_secs,
            json,
        } => {
            if repeat == 0 {
                bail!("repeat must be at least 1");
            }

            let image_info = inspect_image(&image)?;
            let mut config = ProbeConfig::new(image_info, port, baud, out.clone(), repeat);
            config.prompt_timeout = Duration::from_secs(prompt_timeout_secs);
            config.command_timeout = Duration::from_secs(command_timeout_secs);
            config.progress_to_stderr = json;
            let summary = run_rockpro64_usb_probe(&config)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).context("failed to render summary")?
                );
                eprintln!("summary written to {}", out.join("summary.json").display());
            } else {
                print!("{}", render_probe_summary_human(&summary));
                println!("summary written to {}", out.join("summary.json").display());
            }
        }
    }

    Ok(())
}

fn run_validate(command: ValidateCommand) -> Result<()> {
    match command {
        ValidateCommand::Run { plan, out, json } => {
            let plan = load_validation_plan(&plan)?;
            fs::create_dir_all(&out)
                .with_context(|| format!("failed to create {}", out.display()))?;
            let run = materialize_validation_run(plan);
            let run_path = out.join("validation-run.json");
            fs::write(
                &run_path,
                serde_json::to_string_pretty(&run).context("failed to encode validation run")?,
            )
            .with_context(|| format!("failed to write {}", run_path.display()))?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&run)
                        .context("failed to render validation run JSON")?
                );
                eprintln!("validation run written to {}", run_path.display());
            } else {
                print!("{}", render_validation_run_human(&run));
                println!("validation run written to {}", run_path.display());
            }
        }
        ValidateCommand::QemuArm64 {
            plan,
            u_boot,
            qemu,
            out,
            disk,
            disk_interface,
            timeout_secs,
            dry_run,
            json,
        } => {
            let plan = load_validation_plan(&plan)?;
            let timeout_secs = timeout_secs
                .or_else(|| plan.qemu.as_ref().map(|qemu| qemu.timeout_secs))
                .unwrap_or(20);
            let cpu = plan
                .qemu
                .as_ref()
                .and_then(|qemu| qemu.cpu.clone())
                .unwrap_or_else(|| DEFAULT_QEMU_ARM64_CPU.to_owned());
            if timeout_secs == 0 {
                bail!("--timeout-secs must be at least 1");
            }
            let config = QemuArm64RunConfig {
                qemu_binary: qemu,
                u_boot,
                cpu,
                disk,
                disk_interface,
                timeout: Duration::from_secs(timeout_secs),
                out: out.clone(),
                dry_run,
            };
            let run = run_qemu_arm64_validation(plan, &config)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&run)
                        .context("failed to render validation run JSON")?
                );
                eprintln!(
                    "validation run written to {}",
                    out.join("validation-run.json").display()
                );
            } else {
                print!("{}", render_validation_run_human(&run));
                println!(
                    "validation run written to {}",
                    out.join("validation-run.json").display()
                );
            }

            if !dry_run && !run.passed() {
                bail!("QEMU validation failed");
            }
        }
        ValidateCommand::Rockpro64Serial {
            plan,
            port,
            baud,
            out,
            scenarios,
            allow_destructive_spi,
            json,
        } => {
            let plan = load_validation_plan(&plan)?;
            let config = RockPro64SerialRunConfig {
                port,
                baud,
                out: out.clone(),
                selected_scenarios: scenarios,
                allow_destructive_spi,
                idle_sleep: Duration::from_millis(20),
            };
            let run = run_rockpro64_serial_validation(plan, &config)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&run)
                        .context("failed to render validation run JSON")?
                );
                eprintln!(
                    "validation run written to {}",
                    out.join("validation-run.json").display()
                );
            } else {
                print!("{}", render_validation_run_human(&run));
                println!(
                    "validation run written to {}",
                    out.join("validation-run.json").display()
                );
            }

            if !rockpro64_serial_selected_outcomes_passed(&run) {
                bail!("ROCKPro64 serial validation failed");
            }
        }
    }

    Ok(())
}

fn load_validation_plan(path: &Path) -> Result<ValidationPlan> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn materialize_validation_run(plan: ValidationPlan) -> ValidationRun {
    let outcomes = plan
        .scenarios
        .iter()
        .map(|scenario| ValidationOutcome {
            scenario: scenario.name.clone(),
            status: ValidationOutcomeStatus::NotRun,
            executor: Some(ValidationExecutorKind::Deferred),
            evidence: Vec::new(),
            logs: Vec::new(),
            failure: Some("hardware execution is not implemented in this scaffold".into()),
        })
        .collect();

    ValidationRun { plan, outcomes }
}

fn load_manifest(path: &Path) -> Result<FirmwareManifest> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn resolve_flash_artifact(
    manifest_path: &Path,
    manifest: &FirmwareManifest,
    kind: FirmwareArtifactKind,
) -> Result<(PathBuf, FirmwareArtifact)> {
    let artifact = manifest
        .artifact(kind)
        .with_context(|| format!("manifest does not contain artifact {kind}"))?;
    if !artifact.flashable {
        bail!("artifact {kind} is not marked flashable");
    }

    let path = if artifact.path.is_absolute() {
        artifact.path.clone()
    } else {
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&artifact.path)
    };

    Ok((path, artifact.clone()))
}

fn validate_image_against_artifact(
    image: &bootswain_core::ImageInfo,
    artifact: &FirmwareArtifact,
) -> Result<()> {
    if image.size_bytes != artifact.size_bytes {
        bail!(
            "artifact size mismatch for {}: manifest has {}, file has {}",
            artifact.kind,
            artifact.size_bytes,
            image.size_bytes
        );
    }
    if image.sha256 != artifact.sha256 {
        bail!(
            "artifact sha256 mismatch for {}: manifest has {}, file has {}",
            artifact.kind,
            artifact.sha256,
            image.sha256
        );
    }
    if image.compression != artifact.compression {
        bail!(
            "artifact compression mismatch for {}: manifest has {}, file has {}",
            artifact.kind,
            artifact.compression,
            image.compression
        );
    }
    Ok(())
}

fn confirm_device_write(device: &std::path::Path, prompt_to_stderr: bool) -> Result<()> {
    if prompt_to_stderr {
        eprintln!("This will overwrite the target block device.");
        eprintln!("Type the full device path to continue:");
        eprint!("> ");
        io::stderr().flush().context("failed to flush stderr")?;
    } else {
        println!("This will overwrite the target block device.");
        println!("Type the full device path to continue:");
        print!("> ");
        io::stdout().flush().context("failed to flush stdout")?;
    }

    let mut confirmation = String::new();
    io::stdin()
        .read_line(&mut confirmation)
        .context("failed to read confirmation")?;

    if confirmation.trim() != device.display().to_string() {
        bail!("confirmation did not match target device");
    }

    Ok(())
}
