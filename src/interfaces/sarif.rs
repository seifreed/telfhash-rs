use std::path::PathBuf;

use serde::Serialize;

use crate::domain::error::TelfhashError;
use crate::domain::model::{NoSymbolsReason, TelfhashOutcome, TelfhashResult};
use crate::interfaces::legacy::{render_msg, render_telfhash};

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json";

#[derive(Debug, Serialize)]
struct SarifLog<'a> {
    version: &'a str,
    #[serde(rename = "$schema")]
    schema: &'a str,
    runs: Vec<SarifRun<'a>>,
}

#[derive(Debug, Serialize)]
struct SarifRun<'a> {
    tool: SarifTool<'a>,
    results: Vec<SarifResult>,
}

#[derive(Debug, Serialize)]
struct SarifTool<'a> {
    driver: SarifDriver<'a>,
}

#[derive(Debug, Serialize)]
struct SarifDriver<'a> {
    name: &'a str,
    full_name: &'a str,
    version: &'a str,
    information_uri: &'a str,
    rules: Vec<SarifRule<'a>>,
}

#[derive(Debug, Serialize)]
struct SarifRule<'a> {
    id: &'a str,
    name: &'a str,
    short_description: SarifMessage<'a>,
    full_description: SarifMessage<'a>,
}

#[derive(Debug, Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: &'static str,
    level: &'static str,
    message: SarifMessageOwned,
    locations: Vec<SarifLocation>,
    properties: SarifProperties,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "partialFingerprints"
    )]
    partial_fingerprints: Option<SarifFingerprints>,
}

#[derive(Debug, Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
}

#[derive(Debug, Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Debug, Serialize)]
struct SarifMessage<'a> {
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct SarifMessageOwned {
    text: String,
}

#[derive(Debug, Serialize)]
struct SarifProperties {
    status: &'static str,
    telfhash: String,
    msg: String,
}

#[derive(Debug, Serialize)]
struct SarifFingerprints {
    telfhash: String,
}

pub fn render_sarif(results: &[TelfhashResult]) -> Result<String, TelfhashError> {
    let log = SarifLog {
        version: SARIF_VERSION,
        schema: SARIF_SCHEMA,
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "telfhash-rs",
                    full_name: "telfhash-rs ELF similarity hashing",
                    version: env!("CARGO_PKG_VERSION"),
                    information_uri: env!("CARGO_PKG_HOMEPAGE"),
                    rules: rules(),
                },
            },
            results: results.iter().map(sarif_result).collect(),
        }],
    };

    serde_json::to_string(&log).map_err(|error| TelfhashError::Serialization(error.to_string()))
}

fn sarif_result(result: &TelfhashResult) -> SarifResult {
    let rendered_hash = render_telfhash(result);
    let msg = render_msg(result);
    let (rule_id, status, message) = match &result.outcome {
        TelfhashOutcome::Hash(_) if rendered_hash == "tnull" => (
            "TFL002",
            "tnull",
            "telfhash could not produce a stable TLSH digest".to_string(),
        ),
        TelfhashOutcome::Hash(_) => (
            "TFL001",
            "digest",
            format!("telfhash digest computed: {rendered_hash}"),
        ),
        TelfhashOutcome::NoSymbols(NoSymbolsReason::FilteredOut) => (
            "TFL003",
            "no-symbols",
            "no eligible symbols remained after filtering".to_string(),
        ),
        TelfhashOutcome::NoSymbols(NoSymbolsReason::NoCallDestinations) => (
            "TFL004",
            "no-call-destinations",
            "no callable destinations were recovered from executable code".to_string(),
        ),
        TelfhashOutcome::Error(_) => (
            "TFL005",
            "error",
            format!("telfhash analysis failed: {msg}"),
        ),
    };

    SarifResult {
        rule_id,
        level: "note",
        message: SarifMessageOwned { text: message },
        locations: vec![SarifLocation {
            physical_location: SarifPhysicalLocation {
                artifact_location: SarifArtifactLocation {
                    uri: path_uri(&result.file),
                },
            },
        }],
        properties: SarifProperties {
            status,
            telfhash: rendered_hash.clone(),
            msg,
        },
        partial_fingerprints: (rendered_hash != "-" && rendered_hash != "tnull").then_some(
            SarifFingerprints {
                telfhash: rendered_hash,
            },
        ),
    }
}

fn path_uri(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn rules() -> Vec<SarifRule<'static>> {
    vec![
        SarifRule {
            id: "TFL001",
            name: "telfhash-digest",
            short_description: SarifMessage {
                text: "telfhash digest generated",
            },
            full_description: SarifMessage {
                text: "A telfhash TLSH digest was successfully generated for the ELF input.",
            },
        },
        SarifRule {
            id: "TFL002",
            name: "telfhash-tnull",
            short_description: SarifMessage {
                text: "telfhash returned tnull",
            },
            full_description: SarifMessage {
                text: "The input had insufficient information or variance to produce a stable TLSH digest.",
            },
        },
        SarifRule {
            id: "TFL003",
            name: "telfhash-no-symbols",
            short_description: SarifMessage {
                text: "no eligible symbols remained",
            },
            full_description: SarifMessage {
                text: "The ELF input did not expose any eligible function symbols after applying telfhash filters.",
            },
        },
        SarifRule {
            id: "TFL004",
            name: "telfhash-no-call-destinations",
            short_description: SarifMessage {
                text: "no call destinations recovered",
            },
            full_description: SarifMessage {
                text: "The fallback disassembly path did not recover any call destinations from executable code.",
            },
        },
        SarifRule {
            id: "TFL005",
            name: "telfhash-error",
            short_description: SarifMessage {
                text: "telfhash analysis failed",
            },
            full_description: SarifMessage {
                text: "The input could not be processed into a telfhash result.",
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::Value;

    use crate::domain::model::TelfhashResult;

    use super::render_sarif;

    #[test]
    fn serializes_sarif_log_shape() {
        let digest = TelfhashResult::digest(PathBuf::from("sample"), "T1ABC".to_string());
        let value: Value = serde_json::from_str(&render_sarif(&[digest]).unwrap()).unwrap();

        assert_eq!(value["version"], "2.1.0");
        assert_eq!(
            value["runs"][0]["tool"]["driver"]["name"],
            Value::String("telfhash-rs".to_string())
        );
        assert_eq!(value["runs"][0]["results"][0]["ruleId"], "TFL001");
        assert_eq!(
            value["runs"][0]["results"][0]["properties"]["telfhash"],
            "t1abc"
        );
    }
}
