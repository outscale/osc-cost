use std::process::exit;
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};

mod oapi;
mod core;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args = Args::parse();
    if args.debug {
        set_debug_on();
    }

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
    let mut resources = core::Resources::from(oapi_input);
    if debug() {
        eprintln!("info: generated resources has {} vms", resources.vms.len());
    }
    if let Err(error) = resources.compute() {
        eprintln!("error: cannot compute ressource costs: {}", error);
        exit(1);
    }

    match args.format.as_str() {
        "hour" => {
            match resources.cost_per_hour() {
                Ok(cost) => println!("{}", cost),
                Err(error) => {
                    eprintln!("error: cannot compute cost per ressource costs: {}", error);
                    exit(1);
                }
            }
        },
        "month" => {
            match resources.cost_per_month() {
                Ok(cost) => println!("{}", cost),
                Err(error) => {
                    eprintln!("error: cannot compute cost per ressource costs: {}", error);
                    exit(1);
                }
            }
        },
        "json" => {
            match resources.json() {
                Ok(json_details) => println!("{}", json_details),
                Err(error) => {
                    eprintln!("error: cannot compute cost per ressource costs: {}", error);
                    exit(1);
                }
            }
        },
        unknown_format => {
            eprintln!("error: unkown format {}", unknown_format);
            exit(1);
        }
    };
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
}

static DEBUG: AtomicBool = AtomicBool::new(false);

fn set_debug_on() {
    eprintln!("info: debug mode on");
    DEBUG.store(true, Ordering::SeqCst);
}

fn debug() -> bool {
    DEBUG.load(Ordering::SeqCst)
}