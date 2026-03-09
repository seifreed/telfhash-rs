use std::path::Path;

use crate::domain::model::{
    FailureReason, HashValue, NoSymbolsReason, SymbolExtraction, TelfhashOutcome, TelfhashResult,
};
use crate::infrastructure::telemetry::debug;

pub fn emit_debug_report(
    path: &Path,
    extraction: Option<&SymbolExtraction>,
    result: &TelfhashResult,
) {
    if let Some(extraction) = extraction {
        if let Some(elf_class) = &extraction.debug.elf_class {
            eprintln!("{elf_class}");
        }
        if let Some(symbol_table) = &extraction.debug.symbol_table {
            eprintln!("symbol table: {symbol_table}");
        }
        eprintln!("{} symbols found", extraction.debug.symbols_found);
        eprintln!("{} symbols considered", extraction.debug.symbols_considered);
        if let Some(reason) = &extraction.debug.fallback_reason {
            eprintln!("{reason}");
        }
        if extraction.symbols.is_empty() {
            eprintln!("{}: no symbols found", path.display());
        }
    }

    match &result.outcome {
        TelfhashOutcome::Hash(HashValue::NullDigest(_)) => {
            eprintln!("TLSH result: tnull (insufficient length or variance)");
            debug!("null digest reason: insufficient length or variance");
        }
        TelfhashOutcome::NoSymbols(reason) => {
            let reason = match reason {
                NoSymbolsReason::FilteredOut => "no symbols",
                NoSymbolsReason::NoCallDestinations => "no call destinations",
            };
            eprintln!("result: - ({reason})");
        }
        TelfhashOutcome::Error(reason) => {
            let message = match reason {
                FailureReason::InvalidElf => "Could not parse file as ELF",
                FailureReason::UnsupportedArchitecture => "Unsupported ELF architecture",
                FailureReason::Message(message) => message.as_str(),
            };
            eprintln!("result: - ({message})");
        }
        TelfhashOutcome::Hash(HashValue::Digest(_)) => {}
    }
}
