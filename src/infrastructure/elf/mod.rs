use std::fs;
use std::path::Path;

use goblin::elf::Elf;

use crate::domain::error::TelfhashError;
use crate::domain::model::{ExtractionDebug, SymbolExtraction};
use crate::domain::ports::SymbolExtractor;
use crate::infrastructure::telemetry::{debug, info};

mod arch;
mod code_region;
mod disasm_fallback;
mod symbol_table;

use arch::{Architecture, elf_class};
use code_region::find_code_context;
use disasm_fallback::extract_call_destinations;
use symbol_table::extract_symbol_table_symbols;

pub struct GoblinElfSymbolExtractor;

impl SymbolExtractor for GoblinElfSymbolExtractor {
    fn extract_symbols(&self, path: &Path) -> Result<SymbolExtraction, TelfhashError> {
        let bytes = fs::read(path)?;
        let elf = Elf::parse(&bytes).map_err(|_| TelfhashError::InvalidElf)?;
        info!(file = %path.display(), machine = elf.header.e_machine, "parsed ELF file");
        let mut debug = ExtractionDebug {
            elf_class: Some(elf_class(&elf)),
            ..ExtractionDebug::default()
        };

        if let Some((symbols, source_debug)) = extract_symbol_table_symbols(&elf) {
            debug!(
                file = %path.display(),
                symbol_table = ?source_debug.symbol_table,
                symbols_found = source_debug.symbols_found,
                symbols_considered = symbols.len(),
                "using symbol table extraction"
            );
            debug.symbol_table = source_debug.symbol_table;
            debug.symbols_found = source_debug.symbols_found;
            debug.symbols_considered = symbols.len();
            return Ok(SymbolExtraction { symbols, debug });
        }

        let architecture = Architecture::from_machine(elf.header.e_machine)?;
        let context = find_code_context(&elf, &bytes)?;
        let call_destinations = extract_call_destinations(architecture, context)?;
        debug!(
            file = %path.display(),
            symbols_considered = call_destinations.len(),
            "falling back to call destination extraction"
        );
        debug.fallback_reason =
            Some("fallback: no usable symbol table, using call destinations".to_string());
        debug.symbols_considered = call_destinations.len();
        Ok(SymbolExtraction {
            symbols: call_destinations,
            debug,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::arch::Architecture;
    use super::code_region::{
        CodeSelection, SectionCandidate, SegmentCandidate, select_code_region,
    };
    use super::disasm_fallback::collect_call_destinations;

    fn load_instruction_fixture(path: &str) -> Vec<(String, String)> {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let body = fs::read_to_string(Path::new(manifest).join(path)).unwrap();
        body.lines()
            .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
            .map(|line| {
                let (mnemonic, op_str) = line.split_once('|').unwrap();
                (mnemonic.to_string(), op_str.to_string())
            })
            .collect()
    }

    fn call_destinations(path: &str, architecture: Architecture) -> Vec<String> {
        let fixture = load_instruction_fixture(path);
        collect_call_destinations(
            architecture,
            fixture
                .iter()
                .map(|(mnemonic, op_str)| (mnemonic.as_str(), op_str.as_str())),
        )
    }

    #[test]
    fn collects_call_destinations_for_all_supported_architectures() {
        assert_eq!(
            call_destinations("tests/fixtures/disasm/x86.txt", Architecture::X86),
            vec!["401000".to_string(), "402000".to_string()]
        );
        assert_eq!(
            call_destinations("tests/fixtures/disasm/x64.txt", Architecture::X64),
            vec!["401020".to_string(), "402100".to_string()]
        );
        assert_eq!(
            call_destinations("tests/fixtures/disasm/arm.txt", Architecture::Arm),
            vec!["10434".to_string(), "20010".to_string()]
        );
        assert_eq!(
            call_destinations("tests/fixtures/disasm/aarch64.txt", Architecture::Aarch64),
            vec!["400754".to_string(), "401000".to_string()]
        );
        assert_eq!(
            call_destinations("tests/fixtures/disasm/mips.txt", Architecture::Mips),
            vec!["801234".to_string(), "804000".to_string()]
        );
    }

    #[test]
    fn prefers_section_containing_entry() {
        let sections = [SectionCandidate {
            address: 0x401000,
            size: 0x40,
            file_offset: 0x1000,
            name: Some(".init"),
            executable: true,
        }];
        let segments = [SegmentCandidate {
            file_offset: 0x2000,
            file_size: 0x80,
            virtual_address: 0x402000,
            executable: true,
        }];

        let selection = select_code_region(0x401020, 0x400000, &sections, &segments);

        assert_eq!(
            selection,
            Some(CodeSelection::EntrySection {
                file_offset: 0x1000,
                size: 0x40,
                address: 0x401000,
            })
        );
    }

    #[test]
    fn falls_back_to_text_section_when_entry_is_elsewhere() {
        let sections = [SectionCandidate {
            address: 0x403000,
            size: 0x60,
            file_offset: 0x3000,
            name: Some(".text"),
            executable: true,
        }];
        let segments = [SegmentCandidate {
            file_offset: 0x5000,
            file_size: 0x90,
            virtual_address: 0x405000,
            executable: true,
        }];

        let selection = select_code_region(0x401020, 0x400000, &sections, &segments);

        assert_eq!(
            selection,
            Some(CodeSelection::TextSection {
                file_offset: 0x3000,
                size: 0x60,
                address: 0x403000,
            })
        );
    }

    #[test]
    fn falls_back_to_executable_segment_without_text_section() {
        let selection = select_code_region(
            0x401050,
            0x400000,
            &[],
            &[SegmentCandidate {
                file_offset: 0x1200,
                file_size: 0x200,
                virtual_address: 0x401000,
                executable: true,
            }],
        );

        assert_eq!(
            selection,
            Some(CodeSelection::ExecutableSegment {
                file_offset: 0x1200,
                size: 0x200,
                entry_offset: 0x50,
                address: 0x401050,
            })
        );
    }

    #[test]
    fn prefers_executable_text_over_non_executable_text() {
        let sections = [
            SectionCandidate {
                address: 0x402000,
                size: 0x40,
                file_offset: 0x2000,
                name: Some(".text"),
                executable: false,
            },
            SectionCandidate {
                address: 0x403000,
                size: 0x80,
                file_offset: 0x3000,
                name: Some(".text"),
                executable: true,
            },
        ];

        let selection = select_code_region(0x401000, 0x400000, &sections, &[]);

        assert_eq!(
            selection,
            Some(CodeSelection::TextSection {
                file_offset: 0x3000,
                size: 0x80,
                address: 0x403000,
            })
        );
    }

    #[test]
    fn executable_segment_uses_zero_offset_when_entry_is_outside_segment() {
        let selection = select_code_region(
            0x500000,
            0x400000,
            &[],
            &[SegmentCandidate {
                file_offset: 0x1200,
                file_size: 0x200,
                virtual_address: 0x401000,
                executable: true,
            }],
        );

        assert_eq!(
            selection,
            Some(CodeSelection::ExecutableSegment {
                file_offset: 0x1200,
                size: 0x200,
                entry_offset: 0,
                address: 0x500000,
            })
        );
    }

    #[test]
    fn returns_none_when_no_executable_region_exists() {
        assert_eq!(select_code_region(0x401000, 0x400000, &[], &[]), None);
    }
}
