//! Read-only Linux evidence after an EFI handoff. Presence is not qualification.
use anyhow::{Context, Result, bail};
use bootswain_core::Board;
use serde::Serialize;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct PlatformProbe {
    pub schema_version: u16,
    pub board: Board,
    pub compatible: Vec<String>,
    pub kernel_release: String,
    pub efi_booted: bool,
    pub efivars_visible: bool,
    pub runtime_variable_write: &'static str,
    pub boot_counting: &'static str,
    pub watchdog_reset: &'static str,
    pub external_rescue: &'static str,
}

pub fn probe_platform(sys: &Path, proc: &Path, expected: Board) -> Result<PlatformProbe> {
    let compatible = read_bounded(&sys.join("firmware/devicetree/base/compatible"))?
        .split('\0')
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let signature = match expected {
        Board::RaspberryPi5 => "raspberrypi,5-model-b",
        Board::Rock5A => "radxa,rock-5a",
        _ => bail!("platform probe supports Pi 5 and ROCK 5A only"),
    };
    if !compatible.iter().any(|item| item == signature) {
        bail!("device-tree board identity does not match {expected}");
    }
    Ok(PlatformProbe {
        schema_version: 1,
        board: expected,
        compatible,
        kernel_release: read_bounded(&proc.join("sys/kernel/osrelease"))?
            .trim()
            .into(),
        efi_booted: sys.join("firmware/efi").is_dir(),
        efivars_visible: sys.join("firmware/efi/efivars").is_dir(),
        runtime_variable_write: "not-measured",
        boot_counting: "not-measured",
        watchdog_reset: "not-measured",
        external_rescue: "not-measured",
    })
}

fn read_bounded(path: &Path) -> Result<String> {
    let mut data = Vec::new();
    std::fs::File::open(path)
        .with_context(|| format!("read {}", path.display()))?
        .take(4097)
        .read_to_end(&mut data)?;
    if data.len() > 4096 {
        bail!("platform property exceeds 4096 bytes");
    }
    Ok(String::from_utf8(data)?)
}
