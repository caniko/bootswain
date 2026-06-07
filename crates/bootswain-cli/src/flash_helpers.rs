use anyhow::{Context, Result, bail};
use bootswain_core::{FirmwareArtifact, FirmwareArtifactKind, FirmwareManifest, read_json_file};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(crate) fn load_manifest(path: &Path) -> Result<FirmwareManifest> {
    read_json_file(path)
}

pub(crate) fn resolve_flash_artifact(
    manifest_path: &Path,
    manifest: &FirmwareManifest,
    kind: FirmwareArtifactKind,
    allow_non_flashable: bool,
) -> Result<(PathBuf, FirmwareArtifact)> {
    let artifact = manifest
        .artifact(kind)
        .with_context(|| format!("manifest does not contain artifact {kind}"))?;
    if !artifact.flashable && !allow_non_flashable {
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

pub(crate) fn validate_image_against_artifact(
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

pub(crate) fn confirm_device_write(device: &std::path::Path, prompt_to_stderr: bool) -> Result<()> {
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
