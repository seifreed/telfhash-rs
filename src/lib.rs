//! Typed Rust API for ELF `telfhash` generation and grouping.
//!
//! The stable public surface is intentionally small:
//! - [`TelfhashEngine`] orchestrates hashing and grouping.
//! - [`TelfhashOptions`] controls grouping mode and threshold.
//! - [`TelfhashResult`] and [`GroupingResult`] expose domain outcomes.
//!
//! Internal modules such as `application`, `interfaces`, and most of `infrastructure`
//! are part of the crate layout, not the preferred external integration surface.
//!
//! # Examples
//!
//! ```no_run
//! use std::path::PathBuf;
//! use telfhash_rs::{GroupingMode, TelfhashEngine, TelfhashOptions};
//!
//! let engine = TelfhashEngine::new();
//! let options = TelfhashOptions {
//!     grouping_mode: GroupingMode::Compatible,
//!     ..Default::default()
//! };
//!
//! let results = engine
//!     .hash_paths([PathBuf::from("samples/example.elf")], &options)
//!     .unwrap();
//!
//! assert_eq!(results.len(), 1);
//! ```
mod application;
mod domain;
mod infrastructure;
mod interfaces;

use std::path::{Path, PathBuf};

use application::service::TelfhashService;
use infrastructure::elf::GoblinElfSymbolExtractor;
use infrastructure::tlsh::TlshRsHasher;

pub use domain::error::TelfhashError;
pub use domain::model::{
    FailureReason, GroupingMode, GroupingResult, HashValue, NoSymbolsReason, NullDigestReason,
    TelfhashOutcome, TelfhashResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelfhashOptions {
    pub debug: bool,
    pub grouping_mode: GroupingMode,
    pub threshold: u32,
}

impl Default for TelfhashOptions {
    fn default() -> Self {
        Self {
            debug: false,
            grouping_mode: GroupingMode::Compatible,
            threshold: 50,
        }
    }
}

pub struct TelfhashEngine {
    inner: TelfhashService<GoblinElfSymbolExtractor, TlshRsHasher>,
}

impl Default for TelfhashEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TelfhashEngine {
    /// Creates a default engine backed by the built-in ELF extractor and TLSH hasher.
    pub fn new() -> Self {
        Self {
            inner: TelfhashService::new(GoblinElfSymbolExtractor, TlshRsHasher),
        }
    }

    /// Hashes a single ELF-like path and returns a typed outcome.
    pub fn hash_file<P: AsRef<Path>>(
        &self,
        path: P,
        options: &TelfhashOptions,
    ) -> Result<TelfhashResult, TelfhashError> {
        let _ = options.debug;
        Ok(self.inner.hash_file(path.as_ref()))
    }

    /// Hashes multiple paths in input order.
    pub fn hash_paths<P: AsRef<Path>>(
        &self,
        paths: impl IntoIterator<Item = P>,
        options: &TelfhashOptions,
    ) -> Result<Vec<TelfhashResult>, TelfhashError> {
        let paths = paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf())
            .collect::<Vec<PathBuf>>();
        let _ = options.debug;
        Ok(self.inner.hash_files(&paths))
    }

    /// Groups existing hash results using the configured grouping mode.
    pub fn group(
        &self,
        results: &[TelfhashResult],
        options: &TelfhashOptions,
    ) -> Result<GroupingResult, TelfhashError> {
        self.inner
            .group(results, options.threshold, options.grouping_mode)
    }
}

pub fn hash_file<P: AsRef<Path>>(
    path: P,
    options: &TelfhashOptions,
) -> Result<TelfhashResult, TelfhashError> {
    TelfhashEngine::new().hash_file(path, options)
}

pub fn hash_paths<P: AsRef<Path>>(
    paths: impl IntoIterator<Item = P>,
    options: &TelfhashOptions,
) -> Result<Vec<TelfhashResult>, TelfhashError> {
    TelfhashEngine::new().hash_paths(paths, options)
}

pub fn group_results(
    results: &[TelfhashResult],
    options: &TelfhashOptions,
) -> Result<GroupingResult, TelfhashError> {
    TelfhashEngine::new().group(results, options)
}

/// Runs the built-in CLI entrypoint.
pub fn run_cli() -> Result<std::process::ExitCode, TelfhashError> {
    interfaces::cli::run()
}
