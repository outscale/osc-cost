use clap::Parser;
use serde_json::Deserializer;
use std::error;
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};

mod core;
mod oapi;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = Args::parse();
    if args.debug {
        set_debug_on();
    }

    let mut resources: core::Resources;
    if let Some(input_file) = args.input.as_deref() {
        let f = File::open(input_file).expect("Error while opening the file");
        let reader = BufReader::new(f);
        let stream = Deserializer::from_reader(reader).into_iter::<core::Resource>();
        resources = core::Resources {
            resources: Vec::new(),
        };
        for value in stream {
            match value {
                Ok(resource) => resources.resources.push(resource),
                Err(error) => {
                    eprintln!("error while reading input {}", error);
                    exit(1);
                }
            }
        }
    } else {
        let mut oapi_input = match oapi::Input::new(args.profile) {
            Ok(input) => input,
            Err(e) => {
                eprintln!("error: cannot load Outscale API as default: {:?}", e);
                exit(1);
            }
        };
        if let Err(error) = oapi_input.fetch() {
            eprintln!("error: cannot fetch ressources: {:?}", error);
        }
        resources = core::Resources::from(oapi_input);
    }

    if let Err(error) = resources.compute() {
        eprintln!("error: cannot compute ressource costs: {}", error);
        exit(1);
    }

    let output: String;
    match args.format.as_str() {
        "hour" => match resources.cost_per_hour() {
            Ok(cost) => output = format!("{}", cost),
            Err(error) => {
                eprintln!("error: {}", error);
                exit(1);
            }
        },
        "month" => match resources.cost_per_month() {
            Ok(cost) => output = format!("{}", cost),
            Err(error) => {
                eprintln!("error: {}", error);
                exit(1);
            }
        },
        "json" => match resources.json() {
            Ok(json_details) => output = json_details,
            Err(error) => {
                eprintln!("error: {}", error);
                exit(1);
            }
        },
        "csv" => match resources.csv() {
            Ok(csv_details) => output = csv_details,
            Err(error) => {
                eprintln!("error: {}", error);
                exit(1);
            }
        },
        unknown_format => {
            eprintln!("error: unkown format {}", unknown_format);
            exit(1);
        }
    };

    if let Some(output_file) = args.output.as_deref() {
        write_to_file(output_file, output).unwrap_or_else(|error| {
            panic!("Problem writing output to the file: {:?}", error);
        });
    } else {
        println!("{}", output);
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    // Profile name to use in ~/.osc/config.json
    #[arg(long, short = 'p', default_value_t = String::from("default"))]
    profile: String,
    #[arg(long, default_value_t = false)]
    debug: bool,
    #[arg(long, default_value_t = String::from("hour"))]
    format: String,
    #[arg(long, short = 'o')]
    output: Option<String>,
    #[arg(long, short = 'i')]
    input: Option<String>,
}

static DEBUG: AtomicBool = AtomicBool::new(false);

fn set_debug_on() {
    eprintln!("info: debug mode on");
    DEBUG.store(true, Ordering::SeqCst);
}

fn debug() -> bool {
    DEBUG.load(Ordering::SeqCst)
}

fn write_to_file(file_path: &str, data: String) -> Result<(), Box<dyn error::Error>> {
    let path = Path::new(file_path);
    let parent = path.parent().unwrap();

    fs::create_dir_all(parent).unwrap();
    let mut file = File::create(file_path)?;
    file.write_all(data.as_bytes()).unwrap();

    Ok(())
}
