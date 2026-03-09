mod common;
mod compatible;
mod connected;

use crate::domain::error::TelfhashError;
use crate::domain::model::{GroupingResult, TelfhashResult};
use crate::domain::ports::{GroupingPolicy, SimilarityHasher};

pub struct LegacyGroupingPolicy;

pub struct ConnectedComponentsPolicy;

impl GroupingPolicy for LegacyGroupingPolicy {
    fn group(
        &self,
        results: &[TelfhashResult],
        threshold: u32,
        hasher: &dyn SimilarityHasher,
    ) -> Result<GroupingResult, TelfhashError> {
        compatible::group_results(results, threshold, hasher)
    }

    fn name(&self) -> &'static str {
        "compatible"
    }
}

impl GroupingPolicy for ConnectedComponentsPolicy {
    fn group(
        &self,
        results: &[TelfhashResult],
        threshold: u32,
        hasher: &dyn SimilarityHasher,
    ) -> Result<GroupingResult, TelfhashError> {
        connected::group_results(results, threshold, hasher)
    }

    fn name(&self) -> &'static str {
        "connected-components"
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::error::TelfhashError;
    use crate::domain::model::{GroupingResult, HashValue, TelfhashResult};
    use crate::domain::ports::{GroupingPolicy, SimilarityHasher};

    use super::{ConnectedComponentsPolicy, LegacyGroupingPolicy};

    struct ContractHasher;

    impl SimilarityHasher for ContractHasher {
        fn hash_symbols(&self, _symbols: &[String]) -> Result<HashValue, TelfhashError> {
            unreachable!()
        }

        fn distance(&self, left: &str, right: &str) -> Result<u32, TelfhashError> {
            Ok(match (left.as_bytes()[0], right.as_bytes()[0]) {
                (b'a', b'b') | (b'b', b'a') => 1,
                (b'b', b'c') | (b'c', b'b') => 1,
                (b'd', b'e') | (b'e', b'd') => 1,
                _ => 99,
            })
        }
    }

    fn result(name: &str, hash: char) -> TelfhashResult {
        TelfhashResult::digest(PathBuf::from(name), hash.to_string().repeat(72))
    }

    #[test]
    fn legacy_and_native_policies_are_formally_selectable() {
        let results = vec![
            result("a", 'a'),
            result("b", 'b'),
            result("c", 'c'),
            result("d", 'd'),
            result("e", 'e'),
        ];

        let legacy = LegacyGroupingPolicy
            .group(&results, 10, &ContractHasher)
            .unwrap();
        let native = ConnectedComponentsPolicy
            .group(&results, 10, &ContractHasher)
            .unwrap();

        assert_eq!(LegacyGroupingPolicy.name(), "compatible");
        assert_eq!(ConnectedComponentsPolicy.name(), "connected-components");
        assert!(!legacy.grouped.is_empty());
        assert_eq!(
            native,
            GroupingResult {
                grouped: vec![
                    vec!["a", "b", "c"].into_iter().map(PathBuf::from).collect(),
                    vec!["d", "e"].into_iter().map(PathBuf::from).collect(),
                ],
                nogroup: vec![],
            }
        );
    }
}
