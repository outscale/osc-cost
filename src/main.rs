use args::OutputFormat;
use log::error;
use serde_json::Deserializer;
use std::error::{self, Error};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::exit;

mod args;
mod core;
mod oapi;
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = args::parse().expect("unable to parse arguments");

    if args.help_resources {
        print_managed_resources_help();
    } else {
        let mut resources = match args.input {
            Some(input_file) => {
                let reader = BufReader::new(File::open(input_file)?);
                let stream = Deserializer::from_reader(reader).into_iter::<core::Resource>();

                core::Resources {
                    resources: stream
                        .map(|value| value.expect("while reading input"))
                        .collect::<Vec<core::Resource>>(),
                }
            }
            None => {
                let mut oapi_input = oapi::Input::new(args.profile)?;
                oapi_input.filters = args.filter;
                oapi_input.fetch()?;
                core::Resources::from(oapi_input)
            }
        };

        resources.compute()?;

        if args.aggregate {
            resources = resources.aggregate();
        }

        let output = match args.format {
            OutputFormat::Hour => format!("{}", resources.cost_per_hour()?),
            OutputFormat::Month => format!("{}", resources.cost_per_month()?),
            OutputFormat::Json => resources.json()?,
            OutputFormat::Csv => resources.csv()?,
        };

        match args.output {
            Some(output_file) => {
                write_to_file(&output_file, output).unwrap_or_else(|error| {
                    error!("Problem writing output to the file: {:?}", error);
                    exit(1);
                });
            }
            None => {
                println!("{}", output);
            }
        }
    }
    Ok(())
}

fn write_to_file(file_path: &str, data: String) -> Result<(), Box<dyn error::Error>> {
    let path = Path::new(file_path);
    let parent = path.parent().unwrap();

    fs::create_dir_all(parent).unwrap();
    let mut file = File::create(file_path)?;
    file.write_all(data.as_bytes()).unwrap();

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
"#
    );
}
