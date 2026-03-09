use capstone::Insn;
use capstone::arch::arm::ArmOperandType;
use capstone::arch::arm64::Arm64OperandType;
use capstone::arch::mips::MipsOperand;
use capstone::arch::x86::X86OperandType;
use capstone::prelude::*;

use crate::domain::error::TelfhashError;
use crate::infrastructure::elf::arch::Architecture;
use crate::infrastructure::elf::code_region::CodeContext;
use crate::infrastructure::telemetry::debug;

pub(crate) fn extract_call_destinations(
    architecture: Architecture,
    context: CodeContext<'_>,
) -> Result<Vec<String>, TelfhashError> {
    debug!(
        architecture = %architecture.name(),
        start_address = context.address,
        code_len = context.bytes.len(),
        "disassembling executable code"
    );
    let engine = architecture.build_capstone()?;
    let instructions = engine
        .disasm_all(context.bytes, context.address)
        .map_err(|error| TelfhashError::Message(error.to_string()))?;

    let mut destinations = Vec::new();
    for instruction in instructions.iter() {
        if let Some(destination) =
            extract_destination_from_detail(&engine, architecture, instruction).or_else(|| {
                parse_destination_operand(
                    architecture,
                    instruction.mnemonic().unwrap_or_default(),
                    instruction.op_str().unwrap_or_default(),
                )
            })
        {
            push_unique(&mut destinations, destination);
        }
    }

    Ok(destinations)
}

#[cfg(test)]
pub(crate) fn collect_call_destinations<'a>(
    architecture: Architecture,
    instructions: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<String> {
    let mut destinations = Vec::new();

    for (mnemonic, op_str) in instructions {
        if let Some(destination) = parse_destination_operand(architecture, mnemonic, op_str) {
            push_unique(&mut destinations, destination);
        }
    }

    destinations
}

fn extract_destination_from_detail(
    engine: &Capstone,
    architecture: Architecture,
    instruction: &Insn<'_>,
) -> Option<String> {
    let detail = engine.insn_detail(instruction).ok()?;
    let mnemonic = instruction.mnemonic().unwrap_or_default();
    let arch_detail = detail.arch_detail();

    match architecture {
        Architecture::X86 | Architecture::X64 => {
            let detail = arch_detail.x86()?;
            let operand = detail.operands().next()?;
            match operand.op_type {
                X86OperandType::Imm(value) if mnemonic == "call" => Some(format!("{value:x}")),
                _ => None,
            }
        }
        Architecture::Arm => {
            let detail = arch_detail.arm()?;
            let operand = detail.operands().next()?;
            match operand.op_type {
                ArmOperandType::Imm(value) if mnemonic.starts_with("bl") => {
                    Some(format!("{value:x}"))
                }
                _ => None,
            }
        }
        Architecture::Aarch64 => {
            let detail = arch_detail.arm64()?;
            let operand = detail.operands().next()?;
            match operand.op_type {
                Arm64OperandType::Imm(value) if mnemonic == "bl" => Some(format!("{value:x}")),
                _ => None,
            }
        }
        Architecture::Mips => {
            let detail = arch_detail.mips()?;
            let mut operands = detail.operands();
            let first = operands.next()?;
            let second = operands.next()?;
            match (mnemonic, first, second) {
                ("lw", MipsOperand::Reg(reg), MipsOperand::Mem(mem))
                    if engine.reg_name(reg).as_deref() == Some("t9") =>
                {
                    Some(format!("{:x}", mem.disp()))
                }
                _ => None,
            }
        }
    }
}

fn parse_destination_operand(
    architecture: Architecture,
    mnemonic: &str,
    op_str: &str,
) -> Option<String> {
    match architecture {
        Architecture::X86 | Architecture::X64 => (mnemonic == "call" && op_str.starts_with("0x"))
            .then(|| op_str.trim_start_matches("0x").to_string()),
        Architecture::Arm => (mnemonic.starts_with("bl") && op_str.starts_with("#0x"))
            .then(|| op_str.trim_start_matches("#0x").to_string()),
        Architecture::Aarch64 => (mnemonic == "bl" && op_str.starts_with("#0x"))
            .then(|| op_str.trim_start_matches("#0x").to_string()),
        Architecture::Mips => {
            if mnemonic == "lw" && op_str.starts_with("$t9, ") {
                let address = op_str.trim_start_matches("$t9, ");
                Some(
                    address
                        .split('(')
                        .next()
                        .unwrap_or(address)
                        .trim_start_matches("0x")
                        .to_string(),
                )
            } else {
                None
            }
        }
    }
}

fn push_unique(output: &mut Vec<String>, value: String) {
    if !output.contains(&value) {
        output.push(value);
    }
}
