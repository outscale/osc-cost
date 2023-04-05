use log::error;
use osc_cost::core::{Resource, Resources};
use osc_cost::oapi::{Input, Filter};
use args::OutputFormat;
use output::human::human;
use output::json::json;
use output::ods::ods;
use output::prometheus::prometheus;
use serde_json::Deserializer;
use std::error::{self, Error};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::exit;

mod args;
mod output;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = args::parse().expect("unable to parse arguments");

    if args.help_resources {
        print_managed_resources_help();
    } else {
        let mut resources = match args.input {
            Some(input_file) => {
                let reader = BufReader::new(File::open(input_file)?);
                let stream = Deserializer::from_reader(reader).into_iter::<Resource>();

                Resources {
                    resources: stream
                        .map(|value| value.expect("while reading input"))
                        .collect::<Vec<Resource>>(),
                }
            }
            None => {
                let mut oapi_input = Input::new(args.profile)?;
                oapi_input.filters = match args.filter {
                    None => None,
                    Some(f) => Some(Filter{
                        tag_keys: f.filter_tag_key,
                        tag_values: f.filter_tag_value,
                        tags: f.filter_tag,
                        skip_resource: f.skip_resource,
                    })
                };
                oapi_input.fetch()?;
                Resources::from(oapi_input)
            }
        };

        resources.compute()?;

        if args.aggregate {
            resources = resources.aggregate();
        }

        let output = match args.format {
            OutputFormat::Hour => format!("{}", resources.cost_per_hour()?).into_bytes(),
            OutputFormat::Month => format!("{}", resources.cost_per_month()?).into_bytes(),
            OutputFormat::Year => format!("{}", resources.cost_per_year()?).into_bytes(),
            OutputFormat::Json => json(&resources)?.into_bytes(),
            OutputFormat::Prometheus => (prometheus(&resources)?).into_bytes(),
            OutputFormat::Ods => ods(&resources)?,
            OutputFormat::Human => human(&resources.aggregate())?.into_bytes(),
        };

        match args.output {
            Some(output_file) => {
                write_to_file(&output_file, output).unwrap_or_else(|error| {
                    error!("Problem writing output to the file: {:?}", error);
                    exit(1);
                });
            }
            None => {
                println!("{}", String::from_utf8_lossy(&output));
            }
        }
    }
    Ok(())
}

fn write_to_file(file_path: &str, data: Vec<u8>) -> Result<(), Box<dyn error::Error>> {
    let path = Path::new(file_path);
    let parent = path.parent().unwrap();

    fs::create_dir_all(parent).unwrap();
    let mut file = File::create(file_path)?;
    file.write_all(&data).unwrap();

    Ok(())
}

pub fn print_managed_resources_help() {
    println!(
        r#"The following resources are managed by osc-cost:
- Volumes (io1, gp2, standard)
- Snapshots (warning: estimation only, should be the highest price)
- Public Ips
- Nat Services
- Load Balancer
- VPN Connection
- Flexible GPU
- Virtual Machines: including Tina types, AWS-compatible types
- Licenses (included in virtual machines details)
  - Microsoft Windows Server 2019 License (0002)
  - mapr license (0003)
  - Oracle Linux OS Distribution (0004)
  - Microsoft Windows 10 E3 VDA License (0005)
  - Red Hat Enterprise Linux OS Distribution (0006)
  - sql server web (0007)
- Outscale Object Storage
"#
    );
}
