mod parser;
mod profile;
mod runner;
mod serial;
mod validation;

pub use parser::{
    classify_autoboot, extract_last_usb_tree_summary, segment_has_prompt, segment_has_reset,
};
pub use runner::{ProbeConfig, run_rockpro64_usb_probe, run_trial_with_session};
pub use serial::{RealSerial, SerialIo};
pub use validation::{
    ValidationExpectation, ValidationOutcome, ValidationPlan, ValidationReport, ValidationScenario,
    ValidationStatus, load_validation_plan_json, load_validation_report_json,
    rockpro64_usb_validation_plan, write_validation_plan_json, write_validation_report_json,
};
