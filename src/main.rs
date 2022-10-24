mod oapi;

use std::process::exit;
use oapi::OutscaleApiInput;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    // Profile name to use in ~/.osc/config.json
   #[arg(long, short = 'p', default_value_t = String::from("default"))]
   profile: String,
}


fn main() {
    let args = Args::parse();

    let _oapi_input = match OutscaleApiInput::new(args.profile) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("error: cannot load Outscale API as default: {:?}", e);
            exit(1);
        }
    };
    println!("ready to make requests");
}
