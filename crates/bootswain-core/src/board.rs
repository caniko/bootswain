#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Board {
    GenericArm64Qemu,
    #[serde(rename = "raspberry-pi-3-b-plus")]
    RaspberryPi3BPlus,
    RockPro64,
}

impl Board {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GenericArm64Qemu => "generic-arm64-qemu",
            Self::RaspberryPi3BPlus => "raspberry-pi-3-b-plus",
            Self::RockPro64 => "rockpro64",
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Board {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "generic-arm64-qemu" => Ok(Self::GenericArm64Qemu),
            "raspberry-pi-3-b-plus" | "raspberrypi3bplus" | "rpi3bplus" | "rpi-3-b-plus" => {
                Ok(Self::RaspberryPi3BPlus)
            }
            "rockpro64" | "rock-pro64" => Ok(Self::RockPro64),
            _ => Err(format!("unknown board: {value}")),
        }
    }
}
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
