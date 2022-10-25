use std::error;
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;
use std::thread::sleep;
use std::time::Duration;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::Error::ResponseError;
use outscale_api::models::{Vm, ReadVmsResponse};
use outscale_api::apis::vm_api::{read_vms};
use http::status::StatusCode;

static THROTTLING_MIN_WAIT_MS: u64 = 1000;
static THROTTLING_MAX_WAIT_MS: u64 = 10000;

pub struct OutscaleApiInput {
    config: Configuration,
    rng: ThreadRng,
    pub vms: Vec::<Vm>,
}

impl OutscaleApiInput {
    pub fn new(profile_name: String) -> Result<OutscaleApiInput, Box<dyn error::Error>> {
        let config_file = ConfigurationFile::load_default()?;
        Ok(OutscaleApiInput {
            config: config_file.configuration(profile_name)?,
            rng: thread_rng(),
            vms: Vec::new(),
        })
    }

    pub fn fetch(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.fetch_vms()?;
        Ok(())
    }


    fn fetch_vms(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadVmsResponse = loop {
            let response = read_vms(&self.config, None);
            if OutscaleApiInput::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        self.vms = match result.vms {
            None => {
                eprintln!("warning: no vm list provided");
                return Ok(());
            },
            Some(vms) => vms,
        };
        return Ok(())
    }

    fn random_wait(&mut self) {
        let wait_time_ms = self.rng.gen_range(THROTTLING_MIN_WAIT_MS..THROTTLING_MAX_WAIT_MS);
        sleep(Duration::from_millis(wait_time_ms));
    }

    fn is_throttled<T>(result: &Result<ReadVmsResponse, outscale_api::apis::Error<T>>) -> bool {
        match result {
            Ok(_) => false,
            Err(error) => match error {
                ResponseError(resp) => match resp.status {
                    StatusCode::SERVICE_UNAVAILABLE => true,
                    StatusCode::TOO_MANY_REQUESTS => true,
                    _ => false,
                },
                _ => false,
            }
        }
    }
}

