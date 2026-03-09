use std::path::PathBuf;

use crate::application::analysis::{AnalysisRequest, GroupingPlan};
use crate::domain::model::GroupingMode;

use super::output::{HashOutputFormat, OutputRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliRequest {
    pub files: Vec<PathBuf>,
    pub grouping_mode: GroupingMode,
    pub group: bool,
    pub threshold: u32,
    pub output: Option<PathBuf>,
    pub format: Option<HashOutputFormat>,
    pub debug: bool,
}

impl CliRequest {
    pub fn to_analysis_request(&self) -> AnalysisRequest {
        if self.group {
            AnalysisRequest::with_grouping(
                self.files.clone(),
                GroupingPlan::new(self.threshold, self.grouping_mode),
            )
        } else {
            AnalysisRequest::hashes_only(self.files.clone())
        }
    }

    pub fn to_output_request(&self) -> OutputRequest {
        let format = match self.format {
            Some(format) => format,
            None if self.output.is_some() => HashOutputFormat::Tsv,
            None => HashOutputFormat::Plain,
        };

        match &self.output {
            Some(path) => OutputRequest::file(format, path.clone(), self.debug),
            None => OutputRequest::stdout(format, self.debug),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::model::GroupingMode;
    use crate::interfaces::output::{HashOutputFormat, OutputDestination};

    use super::CliRequest;

    #[test]
    fn maps_cli_request_to_analysis_and_output_requests() {
        let cli = CliRequest {
            files: vec![PathBuf::from("a")],
            grouping_mode: GroupingMode::Compatible,
            group: false,
            threshold: 50,
            output: Some(PathBuf::from("out.tsv")),
            format: None,
            debug: true,
        };

        let analysis = cli.to_analysis_request();
        let output = cli.to_output_request();

        assert!(analysis.grouping.is_none());
        assert_eq!(output.format, HashOutputFormat::Tsv);
        assert_eq!(
            output.destination,
            OutputDestination::File(PathBuf::from("out.tsv"))
        );
        assert!(output.debug);
    }
}
