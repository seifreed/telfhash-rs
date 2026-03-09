use std::path::Path;

use crate::domain::error::TelfhashError;
use crate::domain::model::{GroupingResult, HashValue, SymbolExtraction, TelfhashResult};

pub trait SymbolExtractor {
    fn extract_symbols(&self, path: &Path) -> Result<SymbolExtraction, TelfhashError>;
}

pub trait SimilarityHasher {
    fn hash_symbols(&self, symbols: &[String]) -> Result<HashValue, TelfhashError>;
    fn distance(&self, left: &str, right: &str) -> Result<u32, TelfhashError>;
}

pub trait GroupingPolicy {
    fn group(
        &self,
        results: &[TelfhashResult],
        threshold: u32,
        hasher: &dyn SimilarityHasher,
    ) -> Result<GroupingResult, TelfhashError>;

    fn name(&self) -> &'static str;
}
