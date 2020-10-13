//! Platform definitions

use crate::cmake::Setting;
use anyhow::{bail, Error};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

/// A single platform known to the build system
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Platform {
    name: PlatformId,
    /// Supported architectures
    architectures: BTreeSet<Sel4Architecture>,
    /// Variations
    #[serde(rename = "variation", alias = "variant", default)]
    variations: BTreeSet<Variation>,
    #[serde(flatten)]
    setting: Setting,
}

/// A unique platform identifier
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct PlatformId(String);

/// A variation of a particular platform
///
/// Where a platform may refer to multiple compatible architectures, the variation can specify a
/// particular architecture with a certain set of features.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Variation {
    name: VariationId,
    #[serde(flatten)]
    setting: Setting,
}

/// An identifier of a variation within a platform
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct VariationId(String);

/// The choice of a specific platform
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub enum PlatformChoice {
    /// Choose any varaition of a given platform
    ChoosePlatform(PlatformId),
    /// Choose specific varaition of a given platform
    ChooseVariation(PlatformId, VariationId),
}
use PlatformChoice::*;

impl FromStr for PlatformChoice {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.split(":").collect::<Vec<_>>().as_slice() {
            [platform] => Ok(ChoosePlatform(PlatformId(platform.to_string()))),
            [platform, variation] => Ok(ChooseVariation(
                PlatformId(platform.to_string()),
                VariationId(variation.to_string()),
            )),
            _ => bail!("Malformed platform choice: {}", string),
        }
    }
}

impl TryFrom<String> for PlatformChoice {
    type Error = Error;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        string.parse()
    }
}

impl Into<String> for PlatformChoice {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl fmt::Display for PlatformChoice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChoosePlatform(PlatformId(platform)) => write!(f, "{}", platform),
            ChooseVariation(PlatformId(platform), VariationId(variation)) => {
                write!(f, "{}:{}", platform, variation)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum Architecture {
    #[serde(rename = "arm")]
    Arm,
    #[serde(rename = "riscv", alias = "risc-v")]
    RiscV,
    #[serde(rename = "x86")]
    X86,
}
pub use Architecture::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub enum Sel4Architecture {
    #[serde(rename = "aarch32")]
    Aarch32,
    #[serde(rename = "aarch64")]
    Aarch64,
    #[serde(rename = "riscv32")]
    RiscV32,
    #[serde(rename = "riscv64")]
    RiscV64,
    #[serde(rename = "ia32")]
    Ia32,
    #[serde(rename = "x86_64", alias = "amd64", alias = "X64")]
    X86_64,
}
pub use Sel4Architecture::*;

impl Sel4Architecture {
    pub fn architecture(self) -> Architecture {
        match self {
            Aarch32 => Arm,
            Aarch64 => Arm,
            RiscV32 => RiscV,
            RiscV64 => RiscV,
            Ia32 => X86,
            X86_64 => X86,
        }
    }
}

impl FromStr for Sel4Architecture {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "riscv32" => Ok(RiscV32),
            "riscv64" => Ok(RiscV64),
            "aarch32" => Ok(Aarch32),
            "arm_hyp" => Ok(Aarch32),
            "aarch64" => Ok(Aarch64),
            "x86_64" => Ok(X86_64),
            "ia32" => Ok(Ia32),
            _ => bail!("Invalid seL4 architecture: {}", string),
        }
    }
}

impl TryFrom<String> for Sel4Architecture {
    type Error = Error;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        string.parse()
    }
}

impl Into<String> for Sel4Architecture {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl fmt::Display for Sel4Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RiscV64 => write!(f, "riscv64"),
            RiscV32 => write!(f, "riscv32"),
            X86_64 => write!(f, "x86_64"),
            Ia32 => write!(f, "ia32"),
            Aarch32 => write!(f, "aarch32"),
            Aarch64 => write!(f, "aarch64"),
        }
    }
}
