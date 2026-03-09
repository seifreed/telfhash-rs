use serde::Serialize;

use crate::domain::error::TelfhashError;
use crate::domain::model::{FailureReason, HashValue, TelfhashOutcome, TelfhashResult};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct LegacyTelfhashRecord {
    file: String,
    telfhash: String,
    msg: String,
}

pub fn render_telfhash(result: &TelfhashResult) -> String {
    match &result.outcome {
        TelfhashOutcome::Hash(HashValue::Digest(value)) => value.to_ascii_lowercase(),
        TelfhashOutcome::Hash(HashValue::NullDigest(_)) => "tnull".to_string(),
        TelfhashOutcome::NoSymbols(_) | TelfhashOutcome::Error(_) => "-".to_string(),
    }
}

pub fn render_msg(result: &TelfhashResult) -> String {
    match &result.outcome {
        TelfhashOutcome::Error(reason) => legacy_failure_message(reason).to_string(),
        TelfhashOutcome::NoSymbols(_) | TelfhashOutcome::Hash(_) => String::new(),
    }
}

pub fn render_json(results: &[TelfhashResult]) -> Result<String, TelfhashError> {
    let records = results
        .iter()
        .map(|result| LegacyTelfhashRecord {
            file: result.file_display(),
            telfhash: render_telfhash(result),
            msg: render_msg(result),
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&records).map_err(|error| TelfhashError::Serialization(error.to_string()))
}

fn legacy_failure_message(reason: &FailureReason) -> &str {
    reason.message()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use crate::domain::model::{NoSymbolsReason, TelfhashResult};

    use super::{render_json, render_msg, render_telfhash};

    #[test]
    fn maps_digest_tnull_and_error_to_legacy_shape() {
        let digest = TelfhashResult::digest(PathBuf::from("sample"), "T1ABC".to_string());
        let tnull = TelfhashResult::null_digest(PathBuf::from("sample"));
        let error = TelfhashResult::failure(PathBuf::from("broken"), "Could not parse file as ELF");

        assert_eq!(render_telfhash(&digest), "t1abc");
        assert_eq!(render_telfhash(&tnull), "tnull");
        assert_eq!(render_telfhash(&error), "-");
        assert_eq!(render_msg(&error), "Could not parse file as ELF");
    }

    #[test]
    fn serializes_legacy_json_shape() {
        let results = vec![
            TelfhashResult::digest(PathBuf::from("sample"), "T1ABC".to_string()),
            TelfhashResult::no_symbols(PathBuf::from("empty"), NoSymbolsReason::FilteredOut),
        ];

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&render_json(&results).unwrap()).unwrap(),
            json!([
                {"file":"sample","telfhash":"t1abc","msg":""},
                {"file":"empty","telfhash":"-","msg":""}
            ])
        );
    }
}
