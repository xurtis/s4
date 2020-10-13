//! Platform definitions

use serde::Deserialize;
use crate::cmake::Setting;
use std::collections::BTreeSet;

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
pub enum PlatformChoice {
    /// Choose any varaition of a given platform
    ChoosePlatform(PlatformId),
    /// Choose specific varaition of a given platform
    ChooseVariation(PlatformId, VariationId),
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
