mod oapi;

use std::process::exit;
use oapi::OutscaleApiInput;

fn main() {
    println!("Thanks for testing osc-cost, the project is in progress. Feel free to ask questions by opening an issue.");
    let _oapi_input = match OutscaleApiInput::new(None) {
        Ok(input) => input,
        Err(e) => {
            eprintln!("error: cannot load Outscale API as default: {:?}", e);
            exit(1);
        }
    };
    println!("ready to make requests");
}
