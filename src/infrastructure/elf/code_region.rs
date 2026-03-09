use goblin::elf::Elf;
use goblin::elf::program_header::{PF_X, PT_LOAD};
use goblin::elf::section_header::SHF_EXECINSTR;

use crate::domain::error::TelfhashError;

pub(crate) struct CodeContext<'a> {
    pub(crate) address: u64,
    pub(crate) bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeSelection {
    EntrySection {
        file_offset: usize,
        size: usize,
        address: u64,
    },
    TextSection {
        file_offset: usize,
        size: usize,
        address: u64,
    },
    ExecutableSegment {
        file_offset: usize,
        size: usize,
        entry_offset: usize,
        address: u64,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SectionCandidate {
    pub(crate) address: u64,
    pub(crate) size: u64,
    pub(crate) file_offset: usize,
    pub(crate) name: Option<&'static str>,
    pub(crate) executable: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SegmentCandidate {
    pub(crate) file_offset: usize,
    pub(crate) file_size: usize,
    pub(crate) virtual_address: u64,
    pub(crate) executable: bool,
}

pub(crate) fn find_code_context<'a>(
    elf: &Elf<'_>,
    bytes: &'a [u8],
) -> Result<CodeContext<'a>, TelfhashError> {
    let entry = elf.entry;
    let image_base = image_base(elf);
    let sections = elf
        .section_headers
        .iter()
        .filter(|section| section.sh_size > 0)
        .map(|section| SectionCandidate {
            address: section.sh_addr,
            size: section.sh_size,
            file_offset: section.sh_offset as usize,
            name: match elf.shdr_strtab.get_at(section.sh_name) {
                Some(".text") => Some(".text"),
                _ => None,
            },
            executable: section.sh_flags & u64::from(SHF_EXECINSTR) == u64::from(SHF_EXECINSTR),
        })
        .collect::<Vec<_>>();
    let segments = elf
        .program_headers
        .iter()
        .filter(|segment| segment.p_type == PT_LOAD)
        .map(|segment| SegmentCandidate {
            file_offset: segment.p_offset as usize,
            file_size: segment.p_filesz as usize,
            virtual_address: segment.p_vaddr,
            executable: segment.p_flags & PF_X == PF_X,
        })
        .collect::<Vec<_>>();

    match select_code_region(entry, image_base, &sections, &segments) {
        Some(CodeSelection::EntrySection {
            file_offset,
            size,
            address,
        })
        | Some(CodeSelection::TextSection {
            file_offset,
            size,
            address,
        }) => {
            let slice = bytes
                .get(file_offset..file_offset.saturating_add(size))
                .ok_or_else(|| TelfhashError::Message("Section exceeds file bounds".to_string()))?;
            Ok(CodeContext {
                address,
                bytes: slice,
            })
        }
        Some(CodeSelection::ExecutableSegment {
            file_offset,
            size,
            entry_offset,
            address,
        }) => {
            let slice = bytes
                .get(file_offset..file_offset.saturating_add(size))
                .ok_or_else(|| TelfhashError::Message("Segment exceeds file bounds".to_string()))?;
            let slice = slice.get(entry_offset..).unwrap_or(&[]);
            Ok(CodeContext {
                address,
                bytes: slice,
            })
        }
        None => Err(TelfhashError::Message(
            "No executable code section found".to_string(),
        )),
    }
}

pub(crate) fn select_code_region(
    entry: u64,
    image_base: u64,
    sections: &[SectionCandidate],
    segments: &[SegmentCandidate],
) -> Option<CodeSelection> {
    for section in sections {
        let start = section.address;
        let end = start.saturating_add(section.size.saturating_sub(1));
        if section.executable && entry >= start && entry <= end {
            return Some(CodeSelection::EntrySection {
                file_offset: section.file_offset,
                size: section.size as usize,
                address: image_base + section.file_offset as u64,
            });
        }
    }

    for section in sections {
        let start = section.address;
        let end = start.saturating_add(section.size.saturating_sub(1));
        if entry >= start && entry <= end {
            return Some(CodeSelection::EntrySection {
                file_offset: section.file_offset,
                size: section.size as usize,
                address: image_base + section.file_offset as u64,
            });
        }
    }

    if let Some(section) = sections
        .iter()
        .find(|section| section.name == Some(".text") && section.executable)
    {
        return Some(CodeSelection::TextSection {
            file_offset: section.file_offset,
            size: section.size as usize,
            address: image_base + section.file_offset as u64,
        });
    }

    if let Some(section) = sections
        .iter()
        .find(|section| section.name == Some(".text"))
    {
        return Some(CodeSelection::TextSection {
            file_offset: section.file_offset,
            size: section.size as usize,
            address: image_base + section.file_offset as u64,
        });
    }

    for segment in segments {
        if !segment.executable {
            continue;
        }

        return Some(CodeSelection::ExecutableSegment {
            file_offset: segment.file_offset,
            size: segment.file_size,
            entry_offset: entry
                .checked_sub(segment.virtual_address)
                .filter(|offset| (*offset as usize) < segment.file_size)
                .unwrap_or(0) as usize,
            address: entry.max(segment.virtual_address),
        });
    }

    None
}

fn image_base(elf: &Elf<'_>) -> u64 {
    elf.program_headers
        .iter()
        .find(|segment| segment.p_type == PT_LOAD)
        .map(|segment| segment.p_vaddr)
        .unwrap_or(0)
}
