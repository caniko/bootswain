use bootswain_core::{Board, FailureStage};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbeProfile {
    pub(crate) board: Board,
    pub(crate) steps: &'static [ProbeCommandStep],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbeCommandStep {
    pub(crate) stage: FailureStage,
    pub(crate) command: &'static str,
    pub(crate) result_field: ProbeResultField,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ProbeResultField {
    Start,
    Tree,
    Reset,
}

const ROCKPRO64_USB_STEPS: &[ProbeCommandStep] = &[
    ProbeCommandStep {
        stage: FailureStage::UsbStart,
        command: "usb start",
        result_field: ProbeResultField::Start,
    },
    ProbeCommandStep {
        stage: FailureStage::UsbTree1,
        command: "usb tree",
        result_field: ProbeResultField::Tree,
    },
    ProbeCommandStep {
        stage: FailureStage::UsbReset,
        command: "usb reset",
        result_field: ProbeResultField::Reset,
    },
    ProbeCommandStep {
        stage: FailureStage::UsbTree2,
        command: "usb tree",
        result_field: ProbeResultField::Tree,
    },
];

pub(crate) fn rockpro64_usb_profile() -> ProbeProfile {
    ProbeProfile {
        board: Board::RockPro64,
        steps: ROCKPRO64_USB_STEPS,
    }
}
