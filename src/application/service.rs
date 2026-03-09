use std::path::{Path, PathBuf};

use crate::application::analysis::{AnalysisReport, AnalysisRequest, GroupingPlan};
use crate::domain::error::TelfhashError;
use crate::domain::grouping::{ConnectedComponentsPolicy, LegacyGroupingPolicy};
use crate::domain::model::{
    GroupingMode, GroupingResult, HashValue, NoSymbolsReason, SymbolExtraction, TelfhashResult,
};
use crate::domain::ports::{GroupingPolicy, SimilarityHasher, SymbolExtractor};
use crate::infrastructure::telemetry::{debug, info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashInspection {
    pub result: TelfhashResult,
    pub extraction: Option<SymbolExtraction>,
}

impl HashInspection {
    fn with_extraction(result: TelfhashResult, extraction: SymbolExtraction) -> Self {
        Self {
            result,
            extraction: Some(extraction),
        }
    }

    fn without_extraction(result: TelfhashResult) -> Self {
        Self {
            result,
            extraction: None,
        }
    }
}

pub struct TelfhashService<E, H> {
    extractor: E,
    hasher: H,
}

impl<E, H> TelfhashService<E, H>
where
    E: SymbolExtractor,
    H: SimilarityHasher,
{
    pub fn new(extractor: E, hasher: H) -> Self {
        Self { extractor, hasher }
    }

    pub fn inspect_file(&self, path: &Path) -> HashInspection {
        info!(file = %path.display(), "hashing ELF candidate");
        match self.extractor.extract_symbols(path) {
            Ok(extraction) => {
                let result = self.finish_hash(path, extraction.clone());
                HashInspection::with_extraction(result, extraction)
            }
            Err(TelfhashError::InvalidElf) => {
                warn!(file = %path.display(), "could not parse file as ELF");
                HashInspection::without_extraction(TelfhashResult::invalid_elf(path.to_path_buf()))
            }
            Err(TelfhashError::UnsupportedArchitecture) => {
                warn!(file = %path.display(), "unsupported ELF architecture");
                HashInspection::without_extraction(TelfhashResult::unsupported_architecture(
                    path.to_path_buf(),
                ))
            }
            Err(error) => {
                warn!(file = %path.display(), error = %error, "hashing failed");
                HashInspection::without_extraction(TelfhashResult::failure(
                    path.to_path_buf(),
                    error.to_string(),
                ))
            }
        }
    }

    pub fn hash_file(&self, path: &Path) -> TelfhashResult {
        self.inspect_file(path).result
    }

    pub fn inspect_files(&self, paths: &[PathBuf]) -> Vec<HashInspection> {
        paths.iter().map(|path| self.inspect_file(path)).collect()
    }

    pub fn analyze(&self, request: &AnalysisRequest) -> Result<AnalysisReport, TelfhashError> {
        let inspections = self.inspect_files(&request.paths);
        let grouping = self.resolve_grouping(&inspections, request.grouping)?;

        Ok(AnalysisReport::new(inspections, grouping))
    }

    pub fn hash_files(&self, paths: &[PathBuf]) -> Vec<TelfhashResult> {
        paths.iter().map(|path| self.hash_file(path)).collect()
    }

    pub fn group(
        &self,
        results: &[TelfhashResult],
        threshold: u32,
        mode: GroupingMode,
    ) -> Result<GroupingResult, TelfhashError> {
        let policy = self.grouping_policy(mode);
        let _policy_name = policy.name();
        info!(
            threshold,
            grouping_mode = %_policy_name,
            inputs = results.len(),
            "grouping hash results"
        );
        policy.group(results, threshold, &self.hasher)
    }

    fn finish_hash(&self, path: &Path, extraction: SymbolExtraction) -> TelfhashResult {
        let no_symbols_reason = if extraction.debug.fallback_reason.is_some() {
            NoSymbolsReason::NoCallDestinations
        } else {
            NoSymbolsReason::FilteredOut
        };

        let symbols = extraction.symbols;
        if symbols.is_empty() {
            debug!(file = %path.display(), "no symbols available after extraction");
            return TelfhashResult::no_symbols(path.to_path_buf(), no_symbols_reason);
        }

        debug!(file = %path.display(), symbol_count = symbols.len(), "hashing extracted symbols");
        match self.hasher.hash_symbols(&symbols) {
            Ok(HashValue::Digest(hash)) => {
                debug!(file = %path.display(), "generated TLSH digest");
                TelfhashResult::digest(path.to_path_buf(), hash)
            }
            Ok(HashValue::NullDigest(_)) => {
                debug!(file = %path.display(), "TLSH returned TNULL");
                TelfhashResult::null_digest(path.to_path_buf())
            }
            Err(error) => {
                warn!(file = %path.display(), error = %error, "TLSH hashing failed");
                TelfhashResult::failure(path.to_path_buf(), error.to_string())
            }
        }
    }

    fn resolve_grouping(
        &self,
        inspections: &[HashInspection],
        grouping: Option<GroupingPlan>,
    ) -> Result<Option<GroupingResult>, TelfhashError> {
        let Some(grouping) = grouping else {
            return Ok(None);
        };
        if inspections.len() < 2 {
            return Ok(None);
        }

        // Grouping remains a second-stage operation over completed hashes to keep
        // extraction/hash concerns separate from similarity orchestration.
        let results = inspections
            .iter()
            .map(|inspection| inspection.result.clone())
            .collect::<Vec<_>>();
        self.group(&results, grouping.threshold, grouping.mode)
            .map(Some)
    }

    fn grouping_policy(&self, mode: GroupingMode) -> &'static dyn GroupingPolicy {
        static LEGACY: LegacyGroupingPolicy = LegacyGroupingPolicy;
        static CONNECTED: ConnectedComponentsPolicy = ConnectedComponentsPolicy;

        match mode {
            GroupingMode::Compatible => &LEGACY,
            GroupingMode::ConnectedComponents => &CONNECTED,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::application::analysis::{AnalysisRequest, GroupingPlan};
    use crate::domain::error::TelfhashError;
    use crate::domain::model::{
        ExtractionDebug, GroupingMode, HashValue, SymbolExtraction, TelfhashResult,
    };
    use crate::domain::ports::{SimilarityHasher, SymbolExtractor};

    use super::TelfhashService;

    struct StubExtractor;

    impl SymbolExtractor for StubExtractor {
        fn extract_symbols(&self, path: &Path) -> Result<SymbolExtraction, TelfhashError> {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            match name {
                "empty" => Ok(SymbolExtraction {
                    symbols: vec![],
                    debug: ExtractionDebug::default(),
                }),
                "invalid" => Err(TelfhashError::InvalidElf),
                _ => Ok(SymbolExtraction {
                    symbols: vec![name.to_string()],
                    debug: ExtractionDebug::default(),
                }),
            }
        }
    }

    struct StubHasher;

    impl SimilarityHasher for StubHasher {
        fn hash_symbols(&self, symbols: &[String]) -> Result<HashValue, TelfhashError> {
            let symbol = symbols.first().cloned().unwrap_or_default();
            if symbol == "tnull" {
                return Ok(HashValue::NullDigest(
                    crate::domain::model::NullDigestReason::InsufficientInformation,
                ));
            }
            Ok(HashValue::Digest(
                symbol.repeat(72).chars().take(72).collect(),
            ))
        }

        fn distance(&self, left: &str, right: &str) -> Result<u32, TelfhashError> {
            Ok(if left == right { 0 } else { 99 })
        }
    }

    #[test]
    fn analyze_without_grouping_returns_only_inspections() {
        let service = TelfhashService::new(StubExtractor, StubHasher);
        let request = AnalysisRequest::hashes_only(vec![PathBuf::from("alpha")]);

        let report = service.analyze(&request).unwrap();

        assert_eq!(report.inspections.len(), 1);
        assert!(report.grouping.is_none());
    }

    #[test]
    fn analyze_with_grouping_resolves_grouping_only_when_possible() {
        let service = TelfhashService::new(StubExtractor, StubHasher);
        let request = AnalysisRequest::with_grouping(
            vec![PathBuf::from("alpha"), PathBuf::from("beta")],
            GroupingPlan::new(10, GroupingMode::ConnectedComponents),
        );

        let report = service.analyze(&request).unwrap();

        assert!(report.grouping.is_some());
    }

    #[test]
    fn inspect_file_maps_invalid_elf_to_typed_failure() {
        let service = TelfhashService::new(StubExtractor, StubHasher);
        let inspection = service.inspect_file(Path::new("invalid"));

        assert_eq!(
            inspection.result,
            TelfhashResult::invalid_elf(PathBuf::from("invalid"))
        );
        assert!(inspection.extraction.is_none());
    }
}
