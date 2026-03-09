use std::io::Write;
use std::path::PathBuf;

use crate::application::analysis::AnalysisReport;
use crate::application::service::HashInspection;
use crate::domain::error::TelfhashError;
use crate::domain::model::{GroupingResult, TelfhashResult};
use crate::interfaces::debug::emit_debug_report;
use crate::interfaces::legacy::{render_json, render_telfhash};
use crate::interfaces::sarif::render_sarif;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashOutputFormat {
    Plain,
    Tsv,
    Json,
    Sarif,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputDestination {
    Stdout,
    File(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRequest {
    pub format: HashOutputFormat,
    pub destination: OutputDestination,
    pub debug: bool,
}

impl OutputRequest {
    pub fn stdout(format: HashOutputFormat, debug: bool) -> Self {
        Self {
            format,
            destination: OutputDestination::Stdout,
            debug,
        }
    }

    pub fn file(format: HashOutputFormat, path: PathBuf, debug: bool) -> Self {
        Self {
            format,
            destination: OutputDestination::File(path),
            debug,
        }
    }
}

pub struct OutputEmitter;

impl OutputEmitter {
    pub fn emit_report(
        request: &OutputRequest,
        report: &AnalysisReport,
    ) -> Result<(), TelfhashError> {
        let results = report.owned_results();
        Self::emit_hashes(request, &report.inspections, &results)?;
        if let Some(groups) = &report.grouping {
            Self::emit_groups(groups);
        }
        Ok(())
    }

    fn emit_hashes(
        request: &OutputRequest,
        inspections: &[HashInspection],
        results: &[TelfhashResult],
    ) -> Result<(), TelfhashError> {
        if request.debug {
            for inspection in inspections {
                emit_debug_report(
                    &inspection.result.file,
                    inspection.extraction.as_ref(),
                    &inspection.result,
                );
            }
        }

        let body = presenter_for(request.format).render(results)?;
        write_output(&request.destination, &body)
    }

    fn emit_groups(groups: &GroupingResult) {
        for (index, group) in groups.grouped.iter().enumerate() {
            println!("Group {}:", index + 1);
            for file in group {
                println!("    {}", file.display());
            }
        }

        if !groups.nogroup.is_empty() {
            println!("Ungrouped:");
            for file in &groups.nogroup {
                println!("    {}", file.display());
            }
        }

        println!();
    }
}

trait OutputPresenter {
    fn render(&self, results: &[TelfhashResult]) -> Result<String, TelfhashError>;
}

struct PlainPresenter;
struct TsvPresenter;
struct JsonPresenter;
struct SarifPresenter;

impl OutputPresenter for PlainPresenter {
    fn render(&self, results: &[TelfhashResult]) -> Result<String, TelfhashError> {
        Ok(render_plain_hashes(results))
    }
}

impl OutputPresenter for TsvPresenter {
    fn render(&self, results: &[TelfhashResult]) -> Result<String, TelfhashError> {
        Ok(render_tsv_hashes(results))
    }
}

impl OutputPresenter for JsonPresenter {
    fn render(&self, results: &[TelfhashResult]) -> Result<String, TelfhashError> {
        Ok(render_json(results)? + "\n")
    }
}

impl OutputPresenter for SarifPresenter {
    fn render(&self, results: &[TelfhashResult]) -> Result<String, TelfhashError> {
        Ok(render_sarif(results)? + "\n")
    }
}

fn presenter_for(format: HashOutputFormat) -> &'static dyn OutputPresenter {
    static PLAIN: PlainPresenter = PlainPresenter;
    static TSV: TsvPresenter = TsvPresenter;
    static JSON: JsonPresenter = JsonPresenter;
    static SARIF: SarifPresenter = SarifPresenter;

    match format {
        HashOutputFormat::Plain => &PLAIN,
        HashOutputFormat::Tsv => &TSV,
        HashOutputFormat::Json => &JSON,
        HashOutputFormat::Sarif => &SARIF,
    }
}

fn render_plain_hashes(results: &[TelfhashResult]) -> String {
    if results.is_empty() {
        return "\n".to_string();
    }

    let width = results
        .iter()
        .map(TelfhashResult::file_display)
        .map(|display| display.len())
        .max()
        .unwrap_or_default();

    let mut body = String::new();
    for result in results {
        body.push_str(&format!(
            "{:<width$}  {}\n",
            result.file_display(),
            render_telfhash(result),
            width = width
        ));
    }
    body.push('\n');
    body
}

fn render_tsv_hashes(results: &[TelfhashResult]) -> String {
    let body = results
        .iter()
        .map(|result| format!("{}\t{}", result.file_display(), render_telfhash(result)))
        .collect::<Vec<_>>()
        .join("\n");
    body + "\n"
}

fn write_output(destination: &OutputDestination, body: &str) -> Result<(), TelfhashError> {
    match destination {
        OutputDestination::File(path) => std::fs::write(path, body)?,
        OutputDestination::Stdout => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(body.as_bytes())?;
        }
    }
    Ok(())
}
