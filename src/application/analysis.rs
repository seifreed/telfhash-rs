use std::path::PathBuf;

use crate::domain::model::{GroupingMode, GroupingResult, TelfhashResult};

use super::service::HashInspection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupingPlan {
    pub threshold: u32,
    pub mode: GroupingMode,
}

impl GroupingPlan {
    pub fn new(threshold: u32, mode: GroupingMode) -> Self {
        Self { threshold, mode }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisRequest {
    pub paths: Vec<PathBuf>,
    pub grouping: Option<GroupingPlan>,
}

impl AnalysisRequest {
    pub fn hashes_only(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            grouping: None,
        }
    }

    pub fn with_grouping(paths: Vec<PathBuf>, grouping: GroupingPlan) -> Self {
        Self {
            paths,
            grouping: Some(grouping),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisReport {
    pub inspections: Vec<HashInspection>,
    pub grouping: Option<GroupingResult>,
}

impl AnalysisReport {
    pub fn new(inspections: Vec<HashInspection>, grouping: Option<GroupingResult>) -> Self {
        Self {
            inspections,
            grouping,
        }
    }

    pub fn results(&self) -> impl Iterator<Item = &TelfhashResult> {
        self.inspections.iter().map(|inspection| &inspection.result)
    }

    pub fn owned_results(&self) -> Vec<TelfhashResult> {
        self.results().cloned().collect()
    }
}
