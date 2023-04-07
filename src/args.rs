use std::str::FromStr;

use clap::Parser;
use log::error;
use osc_cost::core::Resource;

pub fn parse() -> Option<Args> {
    Args::parse().validate()
}

#[derive(Parser, Debug, Clone)]
pub struct Filter {
    #[arg(long, value_name = "KEY")]
    pub filter_tag_key: Vec<String>,
    #[arg(long, value_name = "VALUE")]
    pub filter_tag_value: Vec<String>,
    #[arg(long, value_name = "KEY=VALUE")]
    pub filter_tag: Vec<String>,
    #[arg(long)]
    pub skip_resource: Vec<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct Drift {
    #[arg(long, default_value_t = false)]
    pub compute_drift: bool,
    #[arg(long)]
    pub from_date: Option<String>,
    #[arg(long)]
    pub to_date: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
pub struct Args {
    // Profile name to use in ~/.osc/config.json
    #[arg(long, short = 'p', default_value_t = String::from("default"))]
    pub profile: String,
    #[arg(value_enum, long)]
    pub source: Option<InputSource>,
    #[arg(value_enum, long, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    #[arg(long, short = 'o')]
    pub output: Option<String>,
    #[arg(long, short = 'i')]
    pub input: Option<String>,
    #[command(flatten)]
    pub filter: Option<Filter>,
    #[arg(long, short = 'a', default_value_t = false)]
    pub aggregate: bool,
    #[arg(long, default_value_t = false)]
    pub help_resources: bool,
    #[command(flatten)]
    pub drift: Drift,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum InputSource {
    Json,
    Api,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Hour,
    Month,
    Year,
    Json,
    Ods,
    Human,
    Prometheus,
}

impl Args {
    fn validate(self) -> Option<Self> {
        let mut err_count = 0;
        err_count += match (&self.input, &self.source) {
            (None, Some(InputSource::Api)) => 0,
            (None, Some(InputSource::Json)) => {
                error!("must use --input argument with when using json data source");
                1
            }
            (_, None) => 0,
            (Some(_), Some(InputSource::Json)) => 0,
            (Some(_), Some(InputSource::Api)) => {
                error!("cannot use Outscale API data source with --input file");
                1
            }
        };
        err_count += match (&self.aggregate, &self.format) {
            (false, _) => 0,
            (true, OutputFormat::Json) => 0,
            (true, OutputFormat::Human) => 0,
            (true, OutputFormat::Ods) => 0,
            (true, OutputFormat::Prometheus) => 0,
            (true, OutputFormat::Hour) => {
                error!("cannot aggregate with hour format");
                1
            }
            (true, OutputFormat::Month) => {
                error!("cannot aggregate with month format");
                1
            }
            (true, OutputFormat::Year) => {
                error!("cannot aggregate with annual format");
                1
            }
        };
        err_count += match (&self.output, &self.format) {
            (Some(_), _) => 0,
            (None, OutputFormat::Ods) => {
                error!("cannot print ods to the standard output");
                1
            }
            _ => 0,
        };

        // Check that the skip_resource with valid resources
        if let Some(filter) = &self.filter {
            for skip_resource in &filter.skip_resource {
                if Resource::from_str(skip_resource).is_err() {
                    error!("cannot skip unknown resources {}", skip_resource);
                    err_count += 1
                }
            }
        }

        err_count += match (&self.drift.compute_drift, &self.format) {
            (false, _) => 0,
            (true, OutputFormat::Json) => 0,
            (true, OutputFormat::Human) => 0,
            (true, _) => {
                error!("cannot use drift with the specified output");
                1
            }
        };

        if self.drift.compute_drift
            && (self.drift.to_date.is_none() || self.drift.from_date.is_none())
        {
            error!("you must specified --from-date and --to-date when computing the drift");
            err_count += 1;
        }

        if err_count > 0 {
            return None;
        }
        Some(self)
    }
}
