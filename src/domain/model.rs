use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashValue {
    Digest(String),
    NullDigest(NullDigestReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullDigestReason {
    InsufficientInformation,
}

impl HashValue {
    pub fn digest(&self) -> Option<&str> {
        match self {
            Self::Digest(value) => Some(value.as_str()),
            Self::NullDigest(_) => None,
        }
    }

    pub fn is_groupable(&self) -> bool {
        matches!(self.digest(), Some(value) if value.len() == 72)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelfhashOutcome {
    Hash(HashValue),
    NoSymbols(NoSymbolsReason),
    Error(FailureReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSymbolsReason {
    FilteredOut,
    NoCallDestinations,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureReason {
    InvalidElf,
    UnsupportedArchitecture,
    Message(String),
}

impl TelfhashOutcome {
    pub fn digest(&self) -> Option<&str> {
        match self {
            Self::Hash(value) => value.digest(),
            Self::NoSymbols(_) | Self::Error(_) => None,
        }
    }

    pub fn is_groupable(&self) -> bool {
        matches!(self, Self::Hash(value) if value.is_groupable())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelfhashResult {
    pub file: PathBuf,
    pub outcome: TelfhashOutcome,
}

impl TelfhashResult {
    pub fn digest(file: PathBuf, digest: String) -> Self {
        Self {
            file,
            outcome: TelfhashOutcome::Hash(HashValue::Digest(digest)),
        }
    }

    pub fn null_digest(file: PathBuf) -> Self {
        Self {
            file,
            outcome: TelfhashOutcome::Hash(HashValue::NullDigest(
                NullDigestReason::InsufficientInformation,
            )),
        }
    }

    pub fn no_symbols(file: PathBuf, reason: NoSymbolsReason) -> Self {
        Self {
            file,
            outcome: TelfhashOutcome::NoSymbols(reason),
        }
    }

    pub fn invalid_elf(file: PathBuf) -> Self {
        Self {
            file,
            outcome: TelfhashOutcome::Error(FailureReason::InvalidElf),
        }
    }

    pub fn unsupported_architecture(file: PathBuf) -> Self {
        Self {
            file,
            outcome: TelfhashOutcome::Error(FailureReason::UnsupportedArchitecture),
        }
    }

    pub fn failure(file: PathBuf, msg: impl Into<String>) -> Self {
        Self {
            file,
            outcome: TelfhashOutcome::Error(FailureReason::Message(msg.into())),
        }
    }

    pub fn is_groupable(&self) -> bool {
        self.outcome.is_groupable()
    }

    pub fn digest_str(&self) -> Option<&str> {
        self.outcome.digest()
    }

    pub fn file_display(&self) -> String {
        portable_path_display(&self.file)
    }
}

impl FailureReason {
    pub fn message(&self) -> &str {
        match self {
            Self::InvalidElf => "Could not parse file as ELF",
            Self::UnsupportedArchitecture => "Unsupported ELF architecture",
            Self::Message(message) => message,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GroupingResult {
    pub grouped: Vec<Vec<PathBuf>>,
    pub nogroup: Vec<PathBuf>,
}

impl GroupingResult {
    pub fn empty() -> Self {
        Self {
            grouped: Vec::new(),
            nogroup: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GroupingMode {
    Compatible,
    ConnectedComponents,
}

impl GroupingMode {
    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::ConnectedComponents => "connected-components",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolExtraction {
    pub symbols: Vec<String>,
    pub debug: ExtractionDebug,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractionDebug {
    pub elf_class: Option<String>,
    pub symbol_table: Option<String>,
    pub symbols_found: usize,
    pub symbols_considered: usize,
    pub fallback_reason: Option<String>,
}

pub fn sort_paths(paths: &mut [PathBuf]) {
    paths.sort_by(|left, right| cmp_paths(left.as_path(), right.as_path()));
}

pub fn cmp_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    left.to_string_lossy().cmp(&right.to_string_lossy())
}

pub fn portable_path_display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        FailureReason, HashValue, NoSymbolsReason, NullDigestReason, TelfhashOutcome,
        TelfhashResult, portable_path_display,
    };

    #[test]
    fn exposes_digest_only_for_real_hashes() {
        let result = TelfhashResult::digest(PathBuf::from("sample"), "T1ABC".to_string());
        let tnull = TelfhashResult::null_digest(PathBuf::from("sample"));

        assert_eq!(result.digest_str(), Some("T1ABC"));
        assert_eq!(tnull.digest_str(), None);
    }

    #[test]
    fn preserves_groupability_and_failure_messages() {
        let no_symbols = TelfhashResult {
            file: PathBuf::from("empty"),
            outcome: TelfhashOutcome::NoSymbols(NoSymbolsReason::NoCallDestinations),
        };
        let unsupported = TelfhashResult {
            file: PathBuf::from("mystery"),
            outcome: TelfhashOutcome::Error(FailureReason::UnsupportedArchitecture),
        };
        let tnull = TelfhashResult {
            file: PathBuf::from("sample"),
            outcome: TelfhashOutcome::Hash(HashValue::NullDigest(
                NullDigestReason::InsufficientInformation,
            )),
        };

        assert!(!no_symbols.is_groupable());
        assert!(!tnull.is_groupable());
        assert_eq!(unsupported.digest_str(), None);
        assert_eq!(
            FailureReason::UnsupportedArchitecture.message(),
            "Unsupported ELF architecture"
        );
    }

    #[test]
    fn normalizes_path_separators_for_legacy_outputs() {
        assert_eq!(
            portable_path_display(std::path::Path::new("tests\\fixtures\\bin\\sample")),
            "tests/fixtures/bin/sample"
        );
    }
}
