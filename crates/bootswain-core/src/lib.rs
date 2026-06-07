//! Shared data models and JSON helpers for Bootswain crates.

mod board;
mod firmware;
mod flash;
mod json;
mod usb_probe;
mod validation;

pub use board::Board;
pub use firmware::{
    BootProtocol, BootTarget, CompressionKind, FirmwareArtifact, FirmwareArtifactKind,
    FirmwareBootPolicy, FirmwareEnvironmentPolicy, FirmwareManifest, FirmwareSourceRevisions,
    FirmwareStorageLayout, FirmwareValidationClaim, ImageInfo,
};
pub use flash::{
    BlockDeviceInfo, BlockDeviceKind, FlashExecutionResult, FlashPlan, FlashRunResult,
    FlashStrategy, FlashVerificationResult,
};
pub use json::{read_json_file, write_pretty_json_file};
pub use usb_probe::{
    AutobootResult, FailureStage, ProbeStepResult, SummaryResult, TrialResult,
    UsbProbeValidationExpectation, UsbProbeValidationOutcome, UsbProbeValidationPlan,
    UsbProbeValidationReport, UsbProbeValidationScenario, UsbProbeValidationStatus,
    load_usb_probe_validation_plan_json, load_usb_probe_validation_report_json,
    rockpro64_usb_validation_plan, write_usb_probe_validation_plan_json,
    write_usb_probe_validation_report_json,
};
pub use validation::{
    QemuDiskInterface, QemuValidationConfig, ValidationExecutorKind, ValidationOutcome,
    ValidationOutcomeStatus, ValidationPlan, ValidationRun, ValidationScenario,
    ValidationScenarioKind, ValidationStep,
};

#[cfg(test)]
mod tests;
