use std::error;
use std::thread::sleep;
use std::time::Duration;
use std::convert::From;
use std::collections::HashMap;
use outscale_api::apis::account_api::read_accounts;
use outscale_api::apis::image_api::read_images;
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;
use http::status::StatusCode;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::Error::ResponseError;
use outscale_api::models::{Vm, ReadVmsRequest, ReadVmsResponse, ReadCatalogResponse, ReadCatalogRequest, CatalogEntry, Image, ReadImagesRequest, FiltersImage, ReadImagesResponse, Account, ReadAccountsResponse, ReadAccountsRequest};
use outscale_api::apis::vm_api::read_vms;
use outscale_api::apis::catalog_api::read_catalog;
use crate::core;
use crate::debug;
use crate::VERSION;

static THROTTLING_MIN_WAIT_MS: u64 = 1000;
static THROTTLING_MAX_WAIT_MS: u64 = 10000;

type ImageId = String;
type VmId = String;

pub struct Input {
    config: Configuration,
    rng: ThreadRng,
    pub vms: HashMap::<VmId, Vm>,
    pub vms_images: HashMap<ImageId, Image>,
    pub catalog: Vec<CatalogEntry>,
    pub account: Option<Account>,
}

impl Input {
    pub fn new(profile_name: String) -> Result<Input, Box<dyn error::Error>> {
        let config_file = ConfigurationFile::load_default()?;
        Ok(Input {
            config: config_file.configuration(profile_name)?,
            rng: thread_rng(),
            vms: HashMap::<VmId, Vm>::new(),
            vms_images: HashMap::<ImageId, Image>::new(),
            catalog: Vec::new(),
            account: None,
        })
    }

    pub fn fetch(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.fetch_vms()?;
        self.fetch_vms_images()?;
        self.fetch_catalog()?;
        self.fetch_account()?;
        Ok(())
    }

    fn fetch_vms(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadVmsResponse = loop {
            let request = ReadVmsRequest::new();
            let response = read_vms(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let vms = match result.vms {
            None => {
                if debug() {
                    eprintln!("warning: no vm list provided");
                }
                return Ok(());
            },
            Some(vms) => vms,
        };
        for vm in vms {
            let vm_id = match &vm.vm_id {
                Some(id) => id,
                None => {
                    if debug() {
                        eprintln!("warning: vm has no id");
                    }
                    continue;
                }
            };
            self.vms.insert(vm_id.clone(), vm);
        }

        if debug() {
            eprintln!("info: fetched {} vms", self.vms.len());
        }
        return Ok(())
    }

    fn fetch_vms_images(&mut self) -> Result<(), Box<dyn error::Error>> {
        // Collect all unique images
        let mut images = Vec::<ImageId>::new();
        for (vm_id, _vm) in &self.vms {
            images.push(vm_id.clone());
        }
        images.dedup();
        let mut filters_image = FiltersImage::new();
        filters_image.image_ids = Some(images);
        let mut request = ReadImagesRequest::new();
        request.filters = Some(Box::new(filters_image));
        let result: ReadImagesResponse = loop {
            let response = read_images(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let images = match result.images {
            None => {
                if debug() {
                    eprintln!("warning: no image list provided");
                }
                return Ok(());
            },
            Some(images) => images,
        };
        for image in images {
            let image_id = image.image_id.clone().unwrap_or(String::from(""));
            self.vms_images.insert(image_id, image);
        }

        if debug() {
            eprintln!("info: fetched {} images used by vms", self.vms_images.len());
        }
        return Ok(())
    }

    fn fetch_catalog(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadCatalogResponse = loop {
            let request = ReadCatalogRequest::new();
            let response = read_catalog(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let catalog = match result.catalog {
            Some(catalog) => catalog,
            None => {
                if debug() {
                    eprintln!("warning: no catalog provided");
                }
                return Ok(());
            },
        };

        self.catalog = match catalog.entries {
            Some(entries) => entries,
            None => {
                if debug() {
                    eprintln!("warning: no catalog entries provided");
                }
                return Ok(());
            }
        };
        if debug() {
            eprintln!("info: fetched {} catalog entries", self.catalog.len());
        }
        return Ok(())
    }

    fn fetch_account(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadAccountsResponse = loop {
            let request = ReadAccountsRequest::new();
            let response = read_accounts(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };


        let accounts = match result.accounts {
            None => {
                if debug() {
                    eprintln!("warning: no account available");
                }
                return Ok(());
            },
            Some(accounts) => accounts,
        };
        self.account = match accounts.iter().next() {
            Some(account) => Some(account.clone()),
            None => {
                if debug() {
                    eprintln!("warning: no account in account list");
                }
                return Ok(());
            }
        };
        if debug() {
            eprintln!("info: fetched account details");
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

    fn is_throttled<S, T>(result: &Result<S, outscale_api::apis::Error<T>>) -> bool {
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

impl From<Input> for core::Resources {
    fn from(input: Input) -> Self {
        let mut resources = core::Resources {
            vms: Vec::new(),
        };

        for (_vm_id, _vm) in &input.vms {
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
                vm_image: None,
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
