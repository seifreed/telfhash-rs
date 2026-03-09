use std::collections::HashSet;
use std::path::PathBuf;

use crate::domain::error::TelfhashError;
use crate::domain::model::{GroupingResult, TelfhashResult};
use crate::domain::ports::SimilarityHasher;

use super::common::{digest, eligible_results, push_path, sort_group};

pub(super) fn group_results(
    results: &[TelfhashResult],
    threshold: u32,
    hasher: &dyn SimilarityHasher,
) -> Result<GroupingResult, TelfhashError> {
    LegacyCompatiblePolicy::build(results, threshold, hasher)?.apply()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SimilarityEdge {
    left: PathBuf,
    right: PathBuf,
    distance: u32,
}

/// Reproduces the original Python grouping heuristic:
/// seed a group from each qualifying pair, expand with directly related pairs,
/// then drop groups that are strict subsets of larger groups.
struct LegacyCompatiblePolicy<'a> {
    eligible: Vec<&'a TelfhashResult>,
    threshold: u32,
    edges: Vec<SimilarityEdge>,
}

impl<'a> LegacyCompatiblePolicy<'a> {
    fn build(
        results: &'a [TelfhashResult],
        threshold: u32,
        hasher: &dyn SimilarityHasher,
    ) -> Result<Self, TelfhashError> {
        let eligible = eligible_results(results);
        let edges = pairwise_edges(&eligible, hasher)?;
        Ok(Self {
            eligible,
            threshold,
            edges,
        })
    }

    fn apply(self) -> Result<GroupingResult, TelfhashError> {
        if self.eligible.len() < 2 {
            return Ok(GroupingResult::empty());
        }

        let grouped = condense(self.expand_seed_groups());
        let grouped_files: HashSet<PathBuf> = grouped
            .iter()
            .flat_map(|group| group.iter().cloned())
            .collect();
        let nogroup = unique_paths_in_order(
            self.eligible
                .iter()
                .map(|result| result.file.clone())
                .filter(|path| !grouped_files.contains(path)),
        );

        Ok(GroupingResult { grouped, nogroup })
    }

    fn expand_seed_groups(&self) -> Vec<Vec<PathBuf>> {
        let mut raw_groups = Vec::new();

        for edge in self.qualifying_edges() {
            let mut group = vec![edge.left.clone(), edge.right.clone()];

            for related in self.qualifying_edges() {
                if related.left == edge.left
                    || related.right == edge.left
                    || related.left == edge.right
                    || related.right == edge.right
                {
                    push_path(&mut group, related.left.clone());
                    push_path(&mut group, related.right.clone());
                }
            }

            sort_group(&mut group);
            if !raw_groups.iter().any(|existing| existing == &group) {
                raw_groups.push(group);
            }
        }

        raw_groups
    }

    fn qualifying_edges(&self) -> impl Iterator<Item = &SimilarityEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.distance <= self.threshold)
    }
}

fn pairwise_edges(
    eligible: &[&TelfhashResult],
    hasher: &dyn SimilarityHasher,
) -> Result<Vec<SimilarityEdge>, TelfhashError> {
    let mut edges = Vec::new();
    for left_index in 0..eligible.len() {
        for right_index in (left_index + 1)..eligible.len() {
            let left = eligible[left_index];
            let right = eligible[right_index];
            edges.push(SimilarityEdge {
                left: left.file.clone(),
                right: right.file.clone(),
                distance: hasher.distance(digest(left), digest(right))?,
            });
        }
    }
    Ok(edges)
}

fn unique_paths_in_order(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for path in paths {
        if seen.insert(path.clone()) {
            unique.push(path);
        }
    }

    unique
}

fn condense(mut groups: Vec<Vec<PathBuf>>) -> Vec<Vec<PathBuf>> {
    groups.sort_by(|left, right| {
        left.len()
            .cmp(&right.len())
            .then_with(|| crate::domain::model::cmp_paths(&left[0], &right[0]))
    });

    let mut condensed = Vec::new();

    for index in 0..groups.len() {
        let item = &groups[index];
        let mut rest = groups
            .iter()
            .enumerate()
            .filter_map(|(other_index, other)| (other_index != index).then_some(other));

        let is_subset = rest.any(|other| is_subset(item, other));
        if !is_subset {
            condensed.push(item.clone());
        }
    }

    condensed
}

fn is_subset(left: &[PathBuf], right: &[PathBuf]) -> bool {
    left.iter().all(|item| right.contains(item))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::error::TelfhashError;
    use crate::domain::ports::SimilarityHasher;

    use super::group_results;
    use crate::domain::model::{HashValue, TelfhashResult};

    struct StubHasher;

    impl SimilarityHasher for StubHasher {
        fn hash_symbols(&self, _symbols: &[String]) -> Result<HashValue, TelfhashError> {
            unreachable!()
        }

        fn distance(&self, left: &str, right: &str) -> Result<u32, TelfhashError> {
            let left = left.chars().next().unwrap_or_default();
            let right = right.chars().next().unwrap_or_default();
            let value = match (left, right) {
                ('A', 'B') | ('B', 'A') => 10,
                ('B', 'C') | ('C', 'B') => 10,
                ('C', 'D') | ('D', 'C') => 10,
                _ => 99,
            };
            Ok(value)
        }
    }

    struct OrderHasher;

    impl SimilarityHasher for OrderHasher {
        fn hash_symbols(&self, _symbols: &[String]) -> Result<HashValue, TelfhashError> {
            unreachable!()
        }

        fn distance(&self, left: &str, right: &str) -> Result<u32, TelfhashError> {
            let score = match (left.as_bytes()[0], right.as_bytes()[0]) {
                (b'a', b'b') | (b'b', b'a') => 1,
                (b'a', b'c') | (b'c', b'a') => 1,
                (b'd', b'e') | (b'e', b'd') => 1,
                _ => 99,
            };
            Ok(score)
        }
    }

    fn result(name: &str, hash: &str) -> TelfhashResult {
        TelfhashResult::digest(PathBuf::from(name), hash.to_string())
    }

    #[test]
    fn groups_transitively_connected_hashes() {
        let results = vec![
            result("a", &"A".repeat(72)),
            result("b", &"B".repeat(72)),
            result("c", &"C".repeat(72)),
            result("d", &"D".repeat(72)),
            result("e", &"E".repeat(72)),
        ];

        let grouped = group_results(&results, 50, &StubHasher).unwrap();

        assert_eq!(
            grouped.grouped,
            vec![
                vec!["a", "b", "c", "d"]
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            ]
        );
        assert_eq!(grouped.nogroup, vec![PathBuf::from("e")]);
    }

    #[test]
    fn preserves_order_of_ungrouped_inputs() {
        let results = vec![
            result("a", &"a".repeat(72)),
            result("b", &"b".repeat(72)),
            result("c", &"c".repeat(72)),
            result("d", &"d".repeat(72)),
            result("e", &"e".repeat(72)),
            result("f", &"f".repeat(72)),
        ];

        let grouped = group_results(&results, 1, &OrderHasher).unwrap();

        assert_eq!(
            grouped.grouped,
            vec![
                vec!["d", "e"]
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>(),
                vec!["a", "b", "c"]
                    .into_iter()
                    .map(PathBuf::from)
                    .collect::<Vec<_>>()
            ]
        );
        assert_eq!(grouped.nogroup, vec![PathBuf::from("f")]);
    }
}
