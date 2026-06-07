use super::{
    DeviceHost, LinuxPaths, RealDeviceHost, execute_flash, inspect_device_with, plan_flash,
    validate_required_device_size, validate_target_device_with, verify_flash,
};
use bootswain_core::{BlockDeviceKind, CompressionKind, FlashPlan, FlashStrategy, ImageInfo};
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

fn make_paths(tmp: &tempfile::TempDir) -> LinuxPaths {
    LinuxPaths {
        sys_class_block: tmp.path().join("sys/class/block"),
        proc_mounts: tmp.path().join("proc/mounts"),
    }
}

fn make_host() -> TestHost {
    TestHost::default()
}

fn create_disk_fixture(tmp: &tempfile::TempDir, disk_name: &str) -> (PathBuf, LinuxPaths) {
    let dev_root = tmp.path().join("dev");
    let sys_class = tmp.path().join("sys/class/block");
    let sys_devices = tmp.path().join("sys/devices/mock");
    let proc_dir = tmp.path().join("proc");
    fs::create_dir_all(&dev_root).expect("dev root");
    fs::create_dir_all(&sys_class).expect("sys class");
    fs::create_dir_all(sys_devices.join(disk_name)).expect("sys target");
    fs::create_dir_all(&proc_dir).expect("proc");
    fs::write(proc_dir.join("mounts"), "").expect("mounts");

    let disk = dev_root.join(disk_name);
    fs::write(&disk, "").expect("disk");
    symlink(sys_devices.join(disk_name), sys_class.join(disk_name)).expect("disk sys symlink");
    fs::write(sys_devices.join(disk_name).join("size"), "4096").expect("size");
    fs::write(sys_devices.join(disk_name).join("removable"), "1").expect("removable");
    fs::create_dir_all(sys_devices.join(disk_name).join("device")).expect("device dir");
    fs::write(
        sys_devices.join(disk_name).join("device/model"),
        "USB Reader\n",
    )
    .expect("model");
    fs::write(
        sys_devices.join(disk_name).join("device/vendor"),
        "Kingston\n",
    )
    .expect("vendor");

    (disk, make_paths(tmp))
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

    let host = make_host().with_block_device(fs::canonicalize(&target).expect("canonical target"));
    let paths = make_paths(&tmp);

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

    let host = make_host()
        .with_block_device(fs::canonicalize(&disk).expect("canonical disk"))
        .with_block_device(fs::canonicalize(&part).expect("canonical partition"));
    let paths = make_paths(&tmp);

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
        info: bootswain_core::BlockDeviceInfo {
            original_path: PathBuf::from("/dev/sdb"),
            canonical_path: PathBuf::from("/dev/sdb"),
            block_name: "sdb".into(),
            kind: BlockDeviceKind::Disk,
            size_bytes: Some(1024),
            removable: Some(true),
            model: None,
            vendor: None,
        },
    };

    let plan = plan_flash(&image, &device, true);
    assert_eq!(plan.strategy, FlashStrategy::ZstdDecompress);
    assert!(plan.dry_run);
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

    let raw_plan = FlashPlan {
        image: ImageInfo {
            path: raw_image.clone(),
            compression: CompressionKind::None,
            size_bytes: 9,
            sha256: String::new(),
        },
        device: bootswain_core::BlockDeviceInfo {
            original_path: raw_target.clone(),
            canonical_path: raw_target.clone(),
            block_name: "raw-device".into(),
            kind: BlockDeviceKind::Unknown,
            size_bytes: None,
            removable: None,
            model: None,
            vendor: None,
        },
        strategy: FlashStrategy::RawCopy,
        dry_run: false,
    };
    let zstd_plan = FlashPlan {
        image: ImageInfo {
            path: zstd_image.clone(),
            compression: CompressionKind::Zstd,
            size_bytes: fs::metadata(&zstd_image).expect("zstd metadata").len(),
            sha256: String::new(),
        },
        device: bootswain_core::BlockDeviceInfo {
            original_path: zstd_target.clone(),
            canonical_path: zstd_target.clone(),
            block_name: "zstd-device".into(),
            kind: BlockDeviceKind::Unknown,
            size_bytes: None,
            removable: None,
            model: None,
            vendor: None,
        },
        strategy: FlashStrategy::ZstdDecompress,
        dry_run: false,
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

    let raw_verify = verify_flash(&raw_plan).expect("raw verify");
    assert!(raw_verify.matched);
    assert_eq!(raw_verify.verified_bytes, 9);

    let zstd_verify = verify_flash(&zstd_plan).expect("zstd verify");
    assert!(zstd_verify.matched);
    assert_eq!(zstd_verify.verified_bytes, 9);
}

#[test]
fn verify_flash_reports_mismatch_and_capacity_errors() {
    let tmp = tempdir().expect("tempdir");
    let image = tmp.path().join("raw.img");
    let target = tmp.path().join("raw-device");
    fs::write(&image, b"expected").expect("write image");
    fs::write(&target, b"different").expect("write target");

    let plan = FlashPlan {
        image: ImageInfo {
            path: image,
            compression: CompressionKind::None,
            size_bytes: 8,
            sha256: String::new(),
        },
        device: bootswain_core::BlockDeviceInfo {
            original_path: target.clone(),
            canonical_path: target,
            block_name: "raw-device".into(),
            kind: BlockDeviceKind::Unknown,
            size_bytes: Some(4),
            removable: None,
            model: None,
            vendor: None,
        },
        strategy: FlashStrategy::RawCopy,
        dry_run: false,
    };

    let verification = verify_flash(&plan).expect("verify mismatch");
    assert!(!verification.matched);

    let error = validate_required_device_size(&plan, 8).expect_err("capacity error");
    assert!(error.to_string().contains("too small"));
}

#[test]
fn verify_flash_reports_truncated_target_as_mismatch() {
    let tmp = tempdir().expect("tempdir");
    let image = tmp.path().join("raw.img");
    let target = tmp.path().join("raw-device");
    fs::write(&image, b"expected").expect("write image");
    fs::write(&target, b"exp").expect("write truncated target");

    let plan = FlashPlan {
        image: ImageInfo {
            path: image,
            compression: CompressionKind::None,
            size_bytes: 8,
            sha256: String::new(),
        },
        device: bootswain_core::BlockDeviceInfo {
            original_path: target.clone(),
            canonical_path: target,
            block_name: "raw-device".into(),
            kind: BlockDeviceKind::Unknown,
            size_bytes: Some(8),
            removable: None,
            model: None,
            vendor: None,
        },
        strategy: FlashStrategy::RawCopy,
        dry_run: false,
    };

    let verification = verify_flash(&plan).expect("verify truncated target");
    assert!(!verification.matched);
    assert_eq!(verification.verified_bytes, 0);
}

#[test]
fn required_device_size_accepts_unknown_or_sufficient_capacity() {
    let image = ImageInfo {
        path: PathBuf::from("/tmp/raw.img"),
        compression: CompressionKind::None,
        size_bytes: 8,
        sha256: String::new(),
    };
    let unknown_device = super::ValidatedBlockDevice {
        info: bootswain_core::BlockDeviceInfo {
            original_path: PathBuf::from("/dev/mock0"),
            canonical_path: PathBuf::from("/dev/mock0"),
            block_name: "mock0".into(),
            kind: BlockDeviceKind::Unknown,
            size_bytes: None,
            removable: None,
            model: None,
            vendor: None,
        },
    };
    let sufficient_device = super::ValidatedBlockDevice {
        info: bootswain_core::BlockDeviceInfo {
            size_bytes: Some(16),
            ..unknown_device.info.clone()
        },
    };

    let unknown_plan = plan_flash(&image, &unknown_device, false);
    validate_required_device_size(&unknown_plan, 16).expect("unknown capacity accepted");

    let sufficient_plan = plan_flash(&image, &sufficient_device, false);
    validate_required_device_size(&sufficient_plan, 16).expect("sufficient capacity accepted");
}

#[test]
fn rejects_virtual_device_kinds() {
    for disk_name in ["loop0", "ram0", "dm-0", "md0"] {
        let tmp = tempdir().expect("tempdir");
        let (disk, paths) = create_disk_fixture(&tmp, disk_name);
        let host = make_host().with_block_device(fs::canonicalize(&disk).expect("canonical"));

        let error =
            validate_target_device_with(&host, &paths, &disk).expect_err("virtual rejected");
        assert!(error.to_string().contains("unsafe virtual block device"));
    }
}

#[test]
fn reads_sysfs_backed_metadata() {
    let tmp = tempdir().expect("tempdir");
    let (disk, paths) = create_disk_fixture(&tmp, "sdb");
    let host = make_host().with_block_device(fs::canonicalize(&disk).expect("canonical"));

    let info = inspect_device_with(&host, &paths, &disk).expect("device info");
    assert_eq!(info.kind, BlockDeviceKind::Disk);
    assert_eq!(info.size_bytes, Some(4096 * 512));
    assert_eq!(info.removable, Some(true));
    assert_eq!(info.model.as_deref(), Some("USB Reader"));
    assert_eq!(info.vendor.as_deref(), Some("Kingston"));
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
