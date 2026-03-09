use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};

use crate::application::service::TelfhashService;
use crate::domain::error::TelfhashError;
use crate::domain::model::GroupingMode;
use crate::infrastructure::elf::GoblinElfSymbolExtractor;
use crate::infrastructure::path::expand_paths;
use crate::infrastructure::telemetry::{info, init_cli_logging, warn};
use crate::infrastructure::tlsh::TlshRsHasher;
use crate::interfaces::cli_mapper::CliRequest;
use crate::interfaces::output::{HashOutputFormat, OutputEmitter};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Tsv,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliGroupingMode {
    Compatible,
    ConnectedComponents,
}

impl From<CliGroupingMode> for GroupingMode {
    fn from(value: CliGroupingMode) -> Self {
        match value {
            CliGroupingMode::Compatible => GroupingMode::Compatible,
            CliGroupingMode::ConnectedComponents => GroupingMode::ConnectedComponents,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "telfhash", version, about = "Generate Trend Micro ELF hashes")]
struct Cli {
    #[arg(short = 'g', long = "group")]
    group: bool,
    #[arg(short = 't', long = "threshold", default_value_t = 50)]
    threshold: u32,
    #[arg(long = "group-mode", value_enum, default_value_t = CliGroupingMode::Compatible)]
    group_mode: CliGroupingMode,
    #[arg(short = 'r', long = "recursive")]
    recursive: bool,
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,
    #[arg(short = 'f', long = "format", value_enum)]
    format: Option<OutputFormat>,
    #[arg(short = 'd', long = "debug")]
    debug: bool,
    #[arg(required = true)]
    files: Vec<String>,
}

pub fn run() -> Result<ExitCode, TelfhashError> {
    init_cli_logging();
    let cli = Cli::parse();
    info!(
        group = cli.group,
        threshold = cli.threshold,
        group_mode = %GroupingMode::from(cli.group_mode).cli_name(),
        recursive = cli.recursive,
        format = ?cli.format,
        debug = cli.debug,
        "starting telfhash CLI"
    );
    let files = expand_paths(&cli.files, cli.recursive)?;
    if files.is_empty() {
        warn!("no files matched the provided input patterns");
        eprintln!("No files found");
        return Ok(ExitCode::from(1));
    }

    let service = TelfhashService::new(GoblinElfSymbolExtractor, TlshRsHasher);
    let request = CliRequest {
        files,
        grouping_mode: cli.group_mode.into(),
        group: cli.group,
        threshold: cli.threshold,
        output: cli.output.clone(),
        format: cli.format.map(Into::into),
        debug: cli.debug,
    };
    let report = service.analyze(&request.to_analysis_request())?;

    OutputEmitter::emit_report(&request.to_output_request(), &report)?;

    Ok(ExitCode::SUCCESS)
}

impl From<OutputFormat> for HashOutputFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Tsv => HashOutputFormat::Tsv,
            OutputFormat::Json => HashOutputFormat::Json,
            OutputFormat::Sarif => HashOutputFormat::Sarif,
        }
    }
}
