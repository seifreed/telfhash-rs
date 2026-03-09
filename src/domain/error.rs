use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelfhashError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse file as ELF")]
    InvalidElf,
    #[error("Unsupported ELF architecture")]
    UnsupportedArchitecture,
    #[error("Invalid glob pattern: {0}")]
    InvalidGlobPattern(String),
    #[error("TLSH generation failed: {0}")]
    TlshGeneration(String),
    #[error("TLSH comparison failed: {0}")]
    TlshComparison(String),
    #[error("Serialization failed: {0}")]
    Serialization(String),
}
