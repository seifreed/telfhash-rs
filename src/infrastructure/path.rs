use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::domain::error::TelfhashError;

pub fn expand_paths(inputs: &[String], recursive: bool) -> Result<Vec<PathBuf>, TelfhashError> {
    let mut expanded = Vec::new();
    let mut seen = HashSet::new();

    for input in inputs {
        let matches = glob::glob(input)
            .map_err(|error| TelfhashError::InvalidGlobPattern(error.to_string()))?;
        let mut found_match = false;

        for entry in matches {
            found_match = true;
            let path = entry.map_err(|error| TelfhashError::Message(error.to_string()))?;
            add_path(&mut expanded, &mut seen, &path, recursive);
        }

        if !found_match {
            let direct = PathBuf::from(input);
            if direct.exists() {
                add_path(&mut expanded, &mut seen, &direct, recursive);
            }
        }
    }

    Ok(expanded)
}

fn add_path(output: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: &Path, recursive: bool) {
    if recursive && path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                push_unique(output, seen, entry.into_path());
            }
        }
        return;
    }

    if path.is_file() {
        push_unique(output, seen, path.to_path_buf());
    }
}

fn push_unique(output: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>, path: PathBuf) {
    if seen.insert(path.clone()) {
        output.push(path);
    }
}
