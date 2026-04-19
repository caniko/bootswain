mod parser;
mod runner;
mod serial;

pub use parser::{
    classify_autoboot, extract_last_usb_tree_summary, segment_has_prompt, segment_has_reset,
};
pub use runner::{ProbeConfig, run_rockpro64_usb_probe, run_trial_with_session};
pub use serial::{RealSerial, SerialIo};
