use anyhow::{Context, Result, bail};
use bootswain_core::{CompressionKind, ImageInfo};
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
    pub original_path: PathBuf,
    pub canonical_path: PathBuf,
    pub block_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashStrategy {
    RawCopy,
    ZstdDecompress,
}

#[derive(Debug, Clone)]
pub struct FlashPlan {
    pub image: ImageInfo,
    pub device: PathBuf,
    pub strategy: FlashStrategy,
}

pub fn validate_target_device(device: &Path) -> Result<ValidatedBlockDevice> {
    validate_target_device_with(&RealDeviceHost, &LinuxPaths::default(), device)
}

pub fn validate_target_device_with<H: DeviceHost>(
    host: &H,
    paths: &LinuxPaths,
    device: &Path,
) -> Result<ValidatedBlockDevice> {
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

    if is_mounted_whole_disk(host, paths, &block_name)? {
        bail!(
            "refusing to write to a disk with mounted filesystems: {}",
            canonical_path.display()
        );
    }

    Ok(ValidatedBlockDevice {
        original_path: device.to_path_buf(),
        canonical_path,
        block_name,
    })
}

pub fn plan_flash(image: &ImageInfo, device: &ValidatedBlockDevice) -> FlashPlan {
    let strategy = match image.compression {
        CompressionKind::None => FlashStrategy::RawCopy,
        CompressionKind::Zstd => FlashStrategy::ZstdDecompress,
    };

    FlashPlan {
        image: image.clone(),
        device: device.canonical_path.clone(),
        strategy,
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
        .open(&plan.device)
        .with_context(|| format!("failed to open device {}", plan.device.display()))?;

    let bytes_written = io::copy(&mut reader, &mut output)
        .with_context(|| format!("failed while writing {}", plan.device.display()))?;
    output
        .flush()
        .with_context(|| format!("failed to flush {}", plan.device.display()))?;
    output
        .sync_all()
        .with_context(|| format!("failed to sync {}", plan.device.display()))?;

    Ok(bytes_written)
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
mod tests {
    use super::{
        DeviceHost, FlashStrategy, LinuxPaths, RealDeviceHost, execute_flash, plan_flash,
        validate_target_device_with,
    };
    use bootswain_core::{CompressionKind, ImageInfo};
    use std::collections::HashSet;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[derive(Debug, Default)]
    struct TestHost {
        block_devices: HashSet<PathBuf>,
    }

    impl TestHost {
        fn with_block_device(mut self, path: PathBuf) -> Self {
            self.block_devices.insert(path);
            self
        }
    }

    impl DeviceHost for TestHost {
        fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
            fs::canonicalize(path)
        }

        fn is_block_device(&self, path: &Path) -> std::io::Result<bool> {
            Ok(self.block_devices.contains(path))
        }

        fn path_exists(&self, path: &Path) -> bool {
            path.exists()
        }

        fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
            fs::read_to_string(path)
        }
    }

    #[test]
    fn rejects_partition_targets() {
        let tmp = tempdir().expect("tempdir");
        let dev_root = tmp.path().join("dev");
        let sys_class = tmp.path().join("sys/class/block");
        let sys_devices = tmp.path().join("sys/devices/mock");
        let proc_dir = tmp.path().join("proc");
        fs::create_dir_all(&dev_root).expect("dev root");
        fs::create_dir_all(&sys_class).expect("sys class");
        fs::create_dir_all(sys_devices.join("sdb/sdb1")).expect("sys target");
        fs::create_dir_all(&proc_dir).expect("proc");
        fs::write(proc_dir.join("mounts"), "").expect("mounts");

        let target = dev_root.join("sdb1");
        fs::write(&target, "").expect("fake device");
        symlink(sys_devices.join("sdb/sdb1"), sys_class.join("sdb1")).expect("sys symlink");
        fs::write(sys_devices.join("sdb/sdb1/partition"), "1").expect("partition marker");

        let host = TestHost::default()
            .with_block_device(fs::canonicalize(&target).expect("canonical target"));
        let paths = LinuxPaths {
            sys_class_block: sys_class,
            proc_mounts: proc_dir.join("mounts"),
        };

        let error =
            validate_target_device_with(&host, &paths, &target).expect_err("partition rejected");
        assert!(error.to_string().contains("whole-disk"));
    }

    #[test]
    fn rejects_disks_with_mounted_filesystems() {
        let tmp = tempdir().expect("tempdir");
        let dev_root = tmp.path().join("dev");
        let sys_class = tmp.path().join("sys/class/block");
        let sys_devices = tmp.path().join("sys/devices/mock");
        let proc_dir = tmp.path().join("proc");
        fs::create_dir_all(&dev_root).expect("dev root");
        fs::create_dir_all(&sys_class).expect("sys class");
        fs::create_dir_all(sys_devices.join("sda/sda1")).expect("sys target");
        fs::create_dir_all(&proc_dir).expect("proc");

        let disk = dev_root.join("sda");
        let part = dev_root.join("sda1");
        fs::write(&disk, "").expect("disk");
        fs::write(&part, "").expect("partition");
        symlink(sys_devices.join("sda"), sys_class.join("sda")).expect("disk sys symlink");
        symlink(sys_devices.join("sda/sda1"), sys_class.join("sda1")).expect("part sys symlink");
        fs::write(sys_devices.join("sda/sda1/partition"), "1").expect("partition marker");
        fs::write(
            proc_dir.join("mounts"),
            format!("{} / ext4 rw 0 0\n", part.display()),
        )
        .expect("mounts");

        let host = TestHost::default()
            .with_block_device(fs::canonicalize(&disk).expect("canonical disk"))
            .with_block_device(fs::canonicalize(&part).expect("canonical partition"));
        let paths = LinuxPaths {
            sys_class_block: sys_class,
            proc_mounts: proc_dir.join("mounts"),
        };

        let error =
            validate_target_device_with(&host, &paths, &disk).expect_err("mounted disk rejected");
        assert!(error.to_string().contains("mounted filesystems"));
    }

    #[test]
    fn plans_strategy_from_image_compression() {
        let image = ImageInfo {
            path: PathBuf::from("/tmp/test.img.zst"),
            compression: CompressionKind::Zstd,
            size_bytes: 1024,
            sha256: "abc".into(),
        };
        let device = super::ValidatedBlockDevice {
            original_path: PathBuf::from("/dev/sdb"),
            canonical_path: PathBuf::from("/dev/sdb"),
            block_name: "sdb".into(),
        };

        let plan = plan_flash(&image, &device);
        assert_eq!(plan.strategy, FlashStrategy::ZstdDecompress);
    }

    #[test]
    fn execute_flash_writes_raw_and_zstd_streams() {
        let tmp = tempdir().expect("tempdir");
        let raw_image = tmp.path().join("raw.img");
        let zstd_image = tmp.path().join("raw.img.zst");
        let raw_target = tmp.path().join("raw-device");
        let zstd_target = tmp.path().join("zstd-device");
        fs::write(&raw_image, b"bootswain").expect("write raw image");
        fs::write(&raw_target, []).expect("write raw target");
        fs::write(&zstd_target, []).expect("write zstd target");

        {
            let input = fs::File::open(&raw_image).expect("open raw image");
            let output = fs::File::create(&zstd_image).expect("create zstd image");
            let mut encoder = zstd::stream::Encoder::new(output, 0).expect("encoder");
            std::io::copy(&mut std::io::BufReader::new(input), &mut encoder).expect("encode");
            encoder.finish().expect("finish");
        }

        let raw_plan = super::FlashPlan {
            image: ImageInfo {
                path: raw_image.clone(),
                compression: CompressionKind::None,
                size_bytes: 9,
                sha256: String::new(),
            },
            device: raw_target.clone(),
            strategy: FlashStrategy::RawCopy,
        };
        let zstd_plan = super::FlashPlan {
            image: ImageInfo {
                path: zstd_image.clone(),
                compression: CompressionKind::Zstd,
                size_bytes: fs::metadata(&zstd_image).expect("zstd metadata").len(),
                sha256: String::new(),
            },
            device: zstd_target.clone(),
            strategy: FlashStrategy::ZstdDecompress,
        };

        execute_flash(&raw_plan).expect("raw flash");
        execute_flash(&zstd_plan).expect("zstd flash");

        assert_eq!(
            fs::read(&raw_target).expect("raw target bytes"),
            b"bootswain"
        );
        assert_eq!(
            fs::read(&zstd_target).expect("zstd target bytes"),
            b"bootswain"
        );
    }

    #[test]
    fn real_device_host_uses_block_device_metadata() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("file");
        fs::write(&path, "").expect("write temp file");

        assert!(
            !RealDeviceHost
                .is_block_device(&path)
                .expect("block device metadata")
        );
    }
}
