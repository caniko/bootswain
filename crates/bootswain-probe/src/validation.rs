use bootswain_core::{
    ImageInfo, UsbProbeValidationExpectation, UsbProbeValidationOutcome, UsbProbeValidationPlan,
    UsbProbeValidationReport, UsbProbeValidationScenario, UsbProbeValidationStatus,
    load_usb_probe_validation_plan_json, load_usb_probe_validation_report_json,
    rockpro64_usb_validation_plan as core_rockpro64_usb_validation_plan,
    write_usb_probe_validation_plan_json, write_usb_probe_validation_report_json,
};
use std::path::Path;

pub type ValidationExpectation = UsbProbeValidationExpectation;
pub type ValidationOutcome = UsbProbeValidationOutcome;
pub type ValidationPlan = UsbProbeValidationPlan;
pub type ValidationReport = UsbProbeValidationReport;
pub type ValidationScenario = UsbProbeValidationScenario;
pub type ValidationStatus = UsbProbeValidationStatus;

pub fn rockpro64_usb_validation_plan(image: ImageInfo) -> ValidationPlan {
    core_rockpro64_usb_validation_plan(image)
}

pub fn write_validation_plan_json(
    path: impl AsRef<Path>,
    plan: &ValidationPlan,
) -> anyhow::Result<()> {
    write_usb_probe_validation_plan_json(path, plan)
}

pub fn load_validation_plan_json(path: impl AsRef<Path>) -> anyhow::Result<ValidationPlan> {
    load_usb_probe_validation_plan_json(path)
}

pub fn write_validation_report_json(
    path: impl AsRef<Path>,
    report: &ValidationReport,
) -> anyhow::Result<()> {
    write_usb_probe_validation_report_json(path, report)
}

pub fn load_validation_report_json(path: impl AsRef<Path>) -> anyhow::Result<ValidationReport> {
    load_usb_probe_validation_report_json(path)
}
