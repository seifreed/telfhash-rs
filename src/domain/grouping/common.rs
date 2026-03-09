use std::path::PathBuf;

use crate::domain::model::{TelfhashResult, sort_paths};

pub(super) fn eligible_results(results: &[TelfhashResult]) -> Vec<&TelfhashResult> {
    results
        .iter()
        .filter(|result| result.is_groupable())
        .collect()
}

pub(super) fn push_path(group: &mut Vec<PathBuf>, path: PathBuf) {
    if !group.contains(&path) {
        group.push(path);
    }
}

pub(super) fn digest(result: &TelfhashResult) -> &str {
    result
        .digest_str()
        .expect("grouping only uses results with concrete TLSH digests")
}

pub(super) fn sort_group(group: &mut Vec<PathBuf>) {
    sort_paths(group);
}
