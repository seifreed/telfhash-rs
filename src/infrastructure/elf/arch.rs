use capstone::Endian;
use capstone::prelude::*;
use goblin::elf::Elf;
use goblin::elf::header::{
    EM_386, EM_AARCH64, EM_ARM, EM_MIPS, EM_MIPS_RS3_LE, EM_MIPS_X, EM_X86_64,
};

use crate::domain::error::TelfhashError;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Architecture {
    X86,
    X64,
    Arm,
    Aarch64,
    Mips,
}

impl Architecture {
    pub(crate) fn from_machine(machine: u16) -> Result<Self, TelfhashError> {
        match machine {
            EM_386 => Ok(Self::X86),
            EM_X86_64 => Ok(Self::X64),
            EM_ARM => Ok(Self::Arm),
            EM_AARCH64 => Ok(Self::Aarch64),
            EM_MIPS | EM_MIPS_RS3_LE | EM_MIPS_X => Ok(Self::Mips),
            _ => Err(TelfhashError::UnsupportedArchitecture),
        }
    }

    pub(crate) fn build_capstone(self) -> Result<Capstone, TelfhashError> {
        let engine = match self {
            Self::X86 => Capstone::new()
                .x86()
                .mode(arch::x86::ArchMode::Mode32)
                .detail(true)
                .build(),
            Self::X64 => Capstone::new()
                .x86()
                .mode(arch::x86::ArchMode::Mode64)
                .detail(true)
                .build(),
            Self::Arm => Capstone::new()
                .arm()
                .mode(arch::arm::ArchMode::Arm)
                .detail(true)
                .build(),
            Self::Aarch64 => Capstone::new()
                .arm64()
                .mode(arch::arm64::ArchMode::Arm)
                .detail(true)
                .build(),
            Self::Mips => Capstone::new()
                .mips()
                .mode(arch::mips::ArchMode::Mips32R6)
                .endian(Endian::Big)
                .detail(true)
                .build(),
        };

        engine.map_err(|error| TelfhashError::Message(error.to_string()))
    }

    #[cfg(feature = "logging")]
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X64 => "x86_64",
            Self::Arm => "arm",
            Self::Aarch64 => "aarch64",
            Self::Mips => "mips",
        }
    }
}

pub(crate) fn elf_class(elf: &Elf<'_>) -> String {
    if elf.is_64 {
        "ELFCLASS64".to_string()
    } else {
        "ELFCLASS32".to_string()
    }
}
