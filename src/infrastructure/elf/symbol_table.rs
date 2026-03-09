use goblin::elf::Elf;
use goblin::elf::section_header::{SHT_DYNSYM, SHT_SYMTAB};
use goblin::elf::sym::{STB_GLOBAL, STT_FUNC, STV_DEFAULT, Sym, st_bind, st_type, st_visibility};

use crate::domain::exclusions::should_exclude;
use crate::domain::model::ExtractionDebug;

pub(crate) fn extract_symbol_table_symbols(
    elf: &Elf<'_>,
) -> Option<(Vec<String>, ExtractionDebug)> {
    let mut use_dynamic = None;

    for section in &elf.section_headers {
        if section.sh_size == 0 {
            continue;
        }

        if section.sh_type == SHT_DYNSYM {
            use_dynamic = Some(true);
            break;
        }

        if section.sh_type == SHT_SYMTAB {
            use_dynamic = Some(false);
            break;
        }
    }

    let use_dynamic = use_dynamic?;
    let mut debug = ExtractionDebug {
        symbol_table: Some(if use_dynamic {
            "SHT_DYNSYM".to_string()
        } else {
            "SHT_SYMTAB".to_string()
        }),
        ..ExtractionDebug::default()
    };

    let mut symbols = if use_dynamic {
        debug.symbols_found = elf.dynsyms.len();
        elf.dynsyms
            .iter()
            .filter_map(|symbol| {
                elf.dynstrtab
                    .get_at(symbol.st_name)
                    .and_then(|name| normalize_symbol(name, symbol))
            })
            .collect::<Vec<_>>()
    } else {
        debug.symbols_found = elf.syms.len();
        elf.syms
            .iter()
            .filter_map(|symbol| {
                elf.strtab
                    .get_at(symbol.st_name)
                    .and_then(|name| normalize_symbol(name, symbol))
            })
            .collect::<Vec<_>>()
    };

    symbols.sort();
    debug.symbols_considered = symbols.len();
    Some((symbols, debug))
}

fn normalize_symbol(name: &str, symbol: Sym) -> Option<String> {
    if st_type(symbol.st_info) != STT_FUNC
        || st_bind(symbol.st_info) != STB_GLOBAL
        || st_visibility(symbol.st_other) != STV_DEFAULT
        || name.is_empty()
        || should_exclude(name)
    {
        return None;
    }

    Some(name.to_ascii_lowercase())
}
