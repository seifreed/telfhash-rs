use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::domain::error::TelfhashError;
use crate::domain::model::{GroupingResult, TelfhashResult, cmp_paths, sort_paths};
use crate::domain::ports::SimilarityHasher;

use super::common::{digest, eligible_results};

pub(super) fn group_results(
    results: &[TelfhashResult],
    threshold: u32,
    hasher: &dyn SimilarityHasher,
) -> Result<GroupingResult, TelfhashError> {
    let eligible = eligible_results(results);
    if eligible.len() < 2 {
        return Ok(GroupingResult::empty());
    }

    let mut graph: HashMap<PathBuf, HashSet<PathBuf>> = eligible
        .iter()
        .map(|result| (result.file.clone(), HashSet::new()))
        .collect();

    for left_index in 0..eligible.len() {
        for right_index in (left_index + 1)..eligible.len() {
            let left = eligible[left_index];
            let right = eligible[right_index];
            if hasher.distance(digest(left), digest(right))? <= threshold {
                graph
                    .get_mut(&left.file)
                    .expect("graph node must exist")
                    .insert(right.file.clone());
                graph
                    .get_mut(&right.file)
                    .expect("graph node must exist")
                    .insert(left.file.clone());
            }
        }
    }

    let mut ordered_files: Vec<PathBuf> = graph.keys().cloned().collect();
    sort_paths(&mut ordered_files);

    let mut visited = HashSet::new();
    let mut grouped = Vec::new();
    let mut nogroup = Vec::new();

    for file in ordered_files {
        if !visited.insert(file.clone()) {
            continue;
        }

        if graph
            .get(&file)
            .is_none_or(|neighbors| neighbors.is_empty())
        {
            nogroup.push(file);
            continue;
        }

        let mut queue = VecDeque::from([file.clone()]);
        let mut component = vec![file];

        while let Some(current) = queue.pop_front() {
            for neighbor in graph.get(&current).expect("graph node must exist") {
                if visited.insert(neighbor.clone()) {
                    queue.push_back(neighbor.clone());
                    component.push(neighbor.clone());
                }
            }
        }

        sort_paths(&mut component);
        grouped.push(component);
    }

    grouped.sort_by(|left, right| cmp_paths(&left[0], &right[0]));
    sort_paths(&mut nogroup);

    Ok(GroupingResult { grouped, nogroup })
}
