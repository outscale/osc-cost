use std::error;
use std::thread::sleep;
use std::time::Duration;
use std::convert::From;
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;
use http::status::StatusCode;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::Error::ResponseError;
use outscale_api::models::{Vm, ReadVmsRequest, ReadVmsResponse};
use outscale_api::apis::vm_api::read_vms;
use crate::core;
use crate::debug;
use crate::VERSION;

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
            let request = ReadVmsRequest::new();
            let response = read_vms(&self.config, Some(request));
            if OutscaleApiInput::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        self.vms = match result.vms {
            None => {
                if debug() {
                    eprintln!("warning: no vm list provided");
                }
                return Ok(());
            },
            Some(vms) => vms,
        };
        if debug() {
            eprintln!("info: fetched {} vms", self.vms.len());
        }
        return Ok(())
    }

    fn random_wait(&mut self) {
        let wait_time_ms = self.rng.gen_range(THROTTLING_MIN_WAIT_MS..THROTTLING_MAX_WAIT_MS);
        if debug() {
            eprintln!("info: call throttled, waiting for {}ms", wait_time_ms);
        }
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

impl From<OutscaleApiInput> for core::Resources {
    fn from(input: OutscaleApiInput) -> Self {
        let mut resources = core::Resources {
            vms: Vec::new(),
        };

        for _vm in input.vms {
            let core_vm = core::Vm {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: None,
                resource_type: None,
                read_date_epoch: None,
                region: None,
                resource_id: None,
                currency: None,
                price_per_hour: None,
                price_per_month: None,
                vm_type: None,
                vm_vcpu_gen: None,
                vm_core_performance: None,
                vm_omi: None,
                vm_product_id: None,
                vm_vcpu: 0,
                vm_ram_gb: 0,
                price_vcpu_per_hour: 0_f32,
                price_ram_gb_per_hour: 0_f32,
                price_product_per_cpu_per_hour: 0_f32,
            };
            resources.vms.push(core_vm);
        }
        return resources;
    }
}
