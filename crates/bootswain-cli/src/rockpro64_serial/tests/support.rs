use super::super::RockPro64SerialRunConfig;
use bootswain_core::{
    Board, BootProtocol, BootTarget, ValidationExecutorKind, ValidationPlan, ValidationScenario,
    ValidationScenarioKind, ValidationStep,
};
use bootswain_probe::SerialIo;
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[derive(Debug, Default)]
pub(super) struct FakeSerial {
    chunks: VecDeque<Vec<u8>>,
    pub(super) writes: Vec<String>,
    clear_drops: VecDeque<usize>,
}

impl FakeSerial {
    pub(super) fn with_chunks(chunks: &[&str]) -> Self {
        Self {
            chunks: chunks
                .iter()
                .map(|chunk| chunk.as_bytes().to_vec())
                .collect(),
            writes: Vec::new(),
            clear_drops: VecDeque::new(),
        }
    }

    pub(super) fn with_chunks_and_clear_drops(chunks: &[&str], clear_drops: &[usize]) -> Self {
        Self {
            chunks: chunks
                .iter()
                .map(|chunk| chunk.as_bytes().to_vec())
                .collect(),
            writes: Vec::new(),
            clear_drops: clear_drops.iter().copied().collect(),
        }
    }
}

impl SerialIo for FakeSerial {
    fn clear(&mut self) -> io::Result<()> {
        let Some(drop_count) = self.clear_drops.pop_front() else {
            return Ok(());
        };
        for _ in 0..drop_count {
            let _ = self.chunks.pop_front();
        }
        Ok(())
    }

    fn read_chunk(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let Some(chunk) = self.chunks.pop_front() else {
            return Ok(0);
        };
        let read = chunk.len().min(buffer.len());
        buffer[..read].copy_from_slice(&chunk[..read]);
        Ok(read)
    }

    fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.writes
            .push(String::from_utf8_lossy(buffer).to_string());
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(super) fn sample_config(
    out: PathBuf,
    selected_scenarios: Vec<String>,
    allow_destructive_spi: bool,
) -> RockPro64SerialRunConfig {
    RockPro64SerialRunConfig {
        port: "/dev/ttyUSB0".into(),
        baud: 115_200,
        out,
        selected_scenarios,
        allow_destructive_spi,
        idle_sleep: Duration::ZERO,
        progress_interval: Duration::ZERO,
        max_step_timeout: None,
        cancellation: Arc::new(AtomicBool::new(false)),
    }
}

pub(super) fn sample_plan() -> ValidationPlan {
    ValidationPlan {
        schema_version: 1,
        board: Board::RockPro64,
        release: "test".into(),
        executor: ValidationExecutorKind::RockPro64Serial,
        qemu: None,
        scenarios: vec![
            ValidationScenario {
                name: "recovery-console".into(),
                kind: ValidationScenarioKind::Prompt,
                target: Some(BootTarget::Spi),
                protocol: Some(BootProtocol::UBootShell),
                artifact: None,
                steps: vec![ValidationStep {
                    name: "prompt".into(),
                    command: None,
                    expect: vec!["=> ".into()],
                    timeout_secs: 1,
                }],
            },
            ValidationScenario {
                name: "bootflow".into(),
                kind: ValidationScenarioKind::BootflowScan,
                target: Some(BootTarget::Sd),
                protocol: None,
                artifact: None,
                steps: vec![ValidationStep {
                    name: "scan".into(),
                    command: Some("bootflow scan".into()),
                    expect: vec!["Scanning for bootflows".into()],
                    timeout_secs: 1,
                }],
            },
            ValidationScenario {
                name: "spi-install".into(),
                kind: ValidationScenarioKind::SpiInstall,
                target: Some(BootTarget::Spi),
                protocol: Some(BootProtocol::UBootShell),
                artifact: None,
                steps: vec![ValidationStep {
                    name: "flash spi".into(),
                    command: Some("run bootswain_flash_spi".into()),
                    expect: vec![
                        "Flashing SPI payload from".into(),
                        "SPI flash complete".into(),
                    ],
                    timeout_secs: 1,
                }],
            },
        ],
    }
}
