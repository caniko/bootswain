use bootswain_core::{FirmwareArtifactKind, QemuDiskInterface};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "bootswain")]
#[command(about = "ROCKPro64-first host-side flash and probe tooling for stock U-Boot")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
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
pub(crate) enum ImageCommand {
    Inspect {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum FlashCommand {
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
        #[arg(long)]
        allow_non_flashable: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum FirmwareCommand {
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
pub(crate) enum ProbeCommand {
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
pub(crate) enum ValidateCommand {
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
        max_step_timeout_secs: Option<u64>,
        #[arg(long)]
        json: bool,
    },
}
