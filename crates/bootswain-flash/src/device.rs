use anyhow::{Context, Result, bail};
use bootswain_core::{
    BlockDeviceInfo, BlockDeviceKind, CompressionKind, FlashPlan, FlashStrategy,
    FlashVerificationResult, ImageInfo,
};
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Write};
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use zstd::stream::read::Decoder;

#[derive(Debug, Clone)]
pub struct LinuxPaths {
    pub sys_class_block: PathBuf,
    pub proc_mounts: PathBuf,
}

impl Default for LinuxPaths {
    fn default() -> Self {
        Self {
            sys_class_block: PathBuf::from("/sys/class/block"),
            proc_mounts: PathBuf::from("/proc/mounts"),
        }
    }
}

pub trait DeviceHost {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn is_block_device(&self, path: &Path) -> io::Result<bool>;
    fn path_exists(&self, path: &Path) -> bool;
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealDeviceHost;

impl DeviceHost for RealDeviceHost {
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }

    fn is_block_device(&self, path: &Path) -> io::Result<bool> {
        Ok(fs::metadata(path)?.file_type().is_block_device())
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedBlockDevice {
    pub info: BlockDeviceInfo,
}

pub fn inspect_device(device: &Path) -> Result<BlockDeviceInfo> {
    inspect_device_with(&RealDeviceHost, &LinuxPaths::default(), device)
}

pub fn validate_target_device(device: &Path) -> Result<ValidatedBlockDevice> {
    validate_target_device_with(&RealDeviceHost, &LinuxPaths::default(), device)
}

pub fn validate_target_device_with<H: DeviceHost>(
    host: &H,
    paths: &LinuxPaths,
    device: &Path,
) -> Result<ValidatedBlockDevice> {
    let info = inspect_device_with(host, paths, device)?;

    match info.kind {
        BlockDeviceKind::Loop
        | BlockDeviceKind::Ram
        | BlockDeviceKind::DeviceMapper
        | BlockDeviceKind::MdRaid => {
            bail!(
                "refusing to write to an unsafe virtual block device ({}): {}",
                info.kind,
                info.canonical_path.display()
            );
        }
        BlockDeviceKind::Disk | BlockDeviceKind::Unknown => {}
    }

    if is_mounted_whole_disk(host, paths, &info.block_name)? {
        bail!(
            "refusing to write to a disk with mounted filesystems: {}",
            info.canonical_path.display()
        );
    }

    Ok(ValidatedBlockDevice { info })
}

pub fn plan_flash(image: &ImageInfo, device: &ValidatedBlockDevice, dry_run: bool) -> FlashPlan {
    let strategy = match image.compression {
        CompressionKind::None => FlashStrategy::RawCopy,
        CompressionKind::Zstd => FlashStrategy::ZstdDecompress,
    };

    FlashPlan {
        image: image.clone(),
        device: device.info.clone(),
        strategy,
        dry_run,
    }
}

pub fn execute_flash(plan: &FlashPlan) -> Result<u64> {
    let input = File::open(&plan.image.path)
        .with_context(|| format!("failed to open image {}", plan.image.path.display()))?;

    let mut reader: Box<dyn Read> = match plan.strategy {
        FlashStrategy::RawCopy => Box::new(BufReader::new(input)),
        FlashStrategy::ZstdDecompress => Box::new(
            Decoder::new(BufReader::new(input))
                .with_context(|| format!("failed to decode {}", plan.image.path.display()))?,
        ),
    };

    let mut output = OpenOptions::new()
        .write(true)
        .open(&plan.device.canonical_path)
        .with_context(|| {
            format!(
                "failed to open device {}",
                plan.device.canonical_path.display()
            )
        })?;

    let bytes_written = io::copy(&mut reader, &mut output).with_context(|| {
        format!(
            "failed while writing {}",
            plan.device.canonical_path.display()
        )
    })?;
    output
        .flush()
        .with_context(|| format!("failed to flush {}", plan.device.canonical_path.display()))?;
    output
        .sync_all()
        .with_context(|| format!("failed to sync {}", plan.device.canonical_path.display()))?;

    Ok(bytes_written)
}

pub fn verify_flash(plan: &FlashPlan) -> Result<FlashVerificationResult> {
    let mut expected = artifact_payload_reader(plan)?;
    let mut actual =
        BufReader::new(File::open(&plan.device.canonical_path).with_context(|| {
            format!(
                "failed to open device {} for verification",
                plan.device.canonical_path.display()
            )
        })?);

    let mut expected_buffer = [0_u8; 64 * 1024];
    let mut actual_buffer = [0_u8; 64 * 1024];
    let mut verified_bytes = 0_u64;

    loop {
        let expected_read = expected.read(&mut expected_buffer).with_context(|| {
            format!(
                "failed to read expected payload from {}",
                plan.image.path.display()
            )
        })?;
        if expected_read == 0 {
            return Ok(FlashVerificationResult {
                image_path: plan.image.path.clone(),
                device_path: plan.device.canonical_path.clone(),
                strategy: plan.strategy,
                verified_bytes,
                matched: true,
            });
        }

        let mut actual_read_total = 0;
        while actual_read_total < expected_read {
            let read = actual
                .read(&mut actual_buffer[actual_read_total..expected_read])
                .with_context(|| {
                    format!(
                        "failed to read {} during verification",
                        plan.device.canonical_path.display()
                    )
                })?;
            if read == 0 {
                return Ok(FlashVerificationResult {
                    image_path: plan.image.path.clone(),
                    device_path: plan.device.canonical_path.clone(),
                    strategy: plan.strategy,
                    verified_bytes,
                    matched: false,
                });
            }
            actual_read_total += read;
        }

        if expected_buffer[..expected_read] != actual_buffer[..expected_read] {
            return Ok(FlashVerificationResult {
                image_path: plan.image.path.clone(),
                device_path: plan.device.canonical_path.clone(),
                strategy: plan.strategy,
                verified_bytes,
                matched: false,
            });
        }

        verified_bytes += expected_read as u64;
    }
}

pub fn validate_required_device_size(plan: &FlashPlan, required_bytes: u64) -> Result<()> {
    if let Some(device_size) = plan.device.size_bytes {
        if device_size < required_bytes {
            bail!(
                "target device is too small: {} bytes available, {} bytes required",
                device_size,
                required_bytes
            );
        }
    }

    Ok(())
}

fn artifact_payload_reader(plan: &FlashPlan) -> Result<Box<dyn Read>> {
    let input = File::open(&plan.image.path)
        .with_context(|| format!("failed to open image {}", plan.image.path.display()))?;

    match plan.strategy {
        FlashStrategy::RawCopy => Ok(Box::new(BufReader::new(input))),
        FlashStrategy::ZstdDecompress => Ok(Box::new(
            Decoder::new(BufReader::new(input))
                .with_context(|| format!("failed to decode {}", plan.image.path.display()))?,
        )),
    }
}

fn inspect_device_with<H: DeviceHost>(
    host: &H,
    paths: &LinuxPaths,
    device: &Path,
) -> Result<BlockDeviceInfo> {
    let canonical_path = host
        .canonicalize(device)
        .with_context(|| format!("failed to resolve target device {}", device.display()))?;

    if !host
        .is_block_device(&canonical_path)
        .with_context(|| format!("failed to stat {}", canonical_path.display()))?
    {
        bail!("target is not a block device: {}", canonical_path.display());
    }

    let block_name = canonical_path
        .file_name()
        .and_then(OsStr::to_str)
        .context("target device path has no usable block-device name")?
        .to_owned();

    let sys_entry = paths.sys_class_block.join(&block_name);
    if !host.path_exists(&sys_entry) {
        bail!(
            "target is not represented in {}: {}",
            paths.sys_class_block.display(),
            canonical_path.display()
        );
    }

    if host.path_exists(&sys_entry.join("partition")) {
        bail!(
            "target must be a whole-disk block device, not a partition: {}",
            canonical_path.display()
        );
    }

    Ok(BlockDeviceInfo {
        original_path: device.to_path_buf(),
        canonical_path,
        block_name: block_name.clone(),
        kind: block_device_kind(&block_name),
        size_bytes: read_sysfs_trimmed(host, &sys_entry.join("size"))
            .and_then(|value| value.parse::<u64>().ok())
            .map(|sectors| sectors.saturating_mul(512)),
        removable: read_sysfs_trimmed(host, &sys_entry.join("removable")).and_then(|value| {
            match value.as_str() {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            }
        }),
        model: read_sysfs_trimmed(host, &sys_entry.join("device/model")),
        vendor: read_sysfs_trimmed(host, &sys_entry.join("device/vendor")),
    })
}

fn block_device_kind(block_name: &str) -> BlockDeviceKind {
    if block_name.starts_with("loop") {
        BlockDeviceKind::Loop
    } else if block_name.starts_with("ram") {
        BlockDeviceKind::Ram
    } else if block_name.starts_with("dm-") {
        BlockDeviceKind::DeviceMapper
    } else if block_name.starts_with("md") {
        BlockDeviceKind::MdRaid
    } else if block_name
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
    {
        BlockDeviceKind::Disk
    } else {
        BlockDeviceKind::Unknown
    }
}

fn read_sysfs_trimmed<H: DeviceHost>(host: &H, path: &Path) -> Option<String> {
    host.read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn is_mounted_whole_disk<H: DeviceHost>(
    host: &H,
    paths: &LinuxPaths,
    target_block: &str,
) -> Result<bool> {
    let mounts = host
        .read_to_string(&paths.proc_mounts)
        .with_context(|| format!("failed to read {}", paths.proc_mounts.display()))?;

    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let Some(source) = fields.next() else {
            continue;
        };

        let source_path = Path::new(source);
        if !source_path.is_absolute() {
            continue;
        }

        let Ok(canonical_source) = host.canonicalize(source_path) else {
            continue;
        };
        let Some(source_name) = canonical_source.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        if whole_disk_name(host, paths, source_name)? == target_block {
            return Ok(true);
        }
    }

    Ok(false)
}

fn whole_disk_name<H: DeviceHost>(
    host: &H,
    paths: &LinuxPaths,
    block_name: &str,
) -> Result<String> {
    let sys_entry = paths.sys_class_block.join(block_name);
    if !host.path_exists(&sys_entry.join("partition")) {
        return Ok(block_name.to_owned());
    }

    let canonical_entry = host
        .canonicalize(&sys_entry)
        .with_context(|| format!("failed to resolve sysfs entry {}", sys_entry.display()))?;

    let parent_name = canonical_entry
        .parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .context("partition sysfs entry has no parent disk name")?;

    Ok(parent_name.to_owned())
}

#[cfg(test)]
mod tests;
