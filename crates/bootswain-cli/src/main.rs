use anyhow::{Context, Result, bail};
use bootswain_flash::{execute_flash, inspect_image, plan_flash, validate_target_device};
use bootswain_probe::{ProbeConfig, run_rockpro64_usb_probe};
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;

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
    Probe {
        #[command(subcommand)]
        command: ProbeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ImageCommand {
    Inspect {
        #[arg(long)]
        image: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum FlashCommand {
    Sd {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        device: PathBuf,
        #[arg(long)]
        yes: bool,
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
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 1)]
        repeat: u32,
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
        Command::Probe { command } => run_probe(command),
    }
}

fn run_image(command: ImageCommand) -> Result<()> {
    match command {
        ImageCommand::Inspect { image } => {
            let info = inspect_image(&image)?;
            println!("path: {}", info.path.display());
            println!("compression: {}", info.compression);
            println!("size-bytes: {}", info.size_bytes);
            println!("sha256: {}", info.sha256);
        }
    }

    Ok(())
}

fn run_flash(command: FlashCommand) -> Result<()> {
    match command {
        FlashCommand::Sd { image, device, yes } => {
            let image_info = inspect_image(&image)?;
            let validated = validate_target_device(&device)?;
            let plan = plan_flash(&image_info, &validated);

            println!("image: {}", image_info.path.display());
            println!("device: {}", validated.canonical_path.display());
            println!("compression: {}", image_info.compression);
            println!("size-bytes: {}", image_info.size_bytes);
            println!("sha256: {}", image_info.sha256);
            println!("strategy: {:?}", plan.strategy);

            if !yes {
                confirm_device_write(&validated.canonical_path)?;
            }

            let bytes_written = execute_flash(&plan)?;
            println!(
                "wrote {} bytes to {}",
                bytes_written,
                validated.canonical_path.display()
            );
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
        } => {
            if repeat == 0 {
                bail!("repeat must be at least 1");
            }

            let image_info = inspect_image(&image)?;
            let config = ProbeConfig::new(image_info, port, baud, out.clone(), repeat);
            let summary = run_rockpro64_usb_probe(&config)?;

            println!(
                "{}",
                serde_json::to_string_pretty(&summary).context("failed to render summary")?
            );
            println!("summary written to {}", out.join("summary.json").display());
        }
    }

    Ok(())
}

fn confirm_device_write(device: &std::path::Path) -> Result<()> {
    println!("This will overwrite the target block device.");
    println!("Type the full device path to continue:");
    print!("> ");
    io::stdout().flush().context("failed to flush stdout")?;

    let mut confirmation = String::new();
    io::stdin()
        .read_line(&mut confirmation)
        .context("failed to read confirmation")?;

    if confirmation.trim() != device.display().to_string() {
        bail!("confirmation did not match target device");
    }

    Ok(())
}
