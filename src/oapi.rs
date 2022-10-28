use std::error;
use std::thread::sleep;
use std::time::Duration;
use std::convert::From;
use std::collections::HashMap;
use outscale_api::apis::subregion_api::read_subregions;
use rand::{thread_rng, Rng};
use rand::rngs::ThreadRng;
use http::status::StatusCode;
use chrono::{DateTime, Utc};
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::Error::ResponseError;
use outscale_api::models::{Vm, ReadVmsRequest, ReadVmsResponse, ReadCatalogResponse, ReadCatalogRequest, CatalogEntry, Image, ReadImagesRequest, FiltersImage, ReadImagesResponse, Account, ReadAccountsResponse, ReadAccountsRequest, ReadSubnetsResponse, ReadSubregionsRequest, ReadSubregionsResponse};
use outscale_api::apis::vm_api::read_vms;
use outscale_api::apis::catalog_api::read_catalog;
use outscale_api::apis::account_api::read_accounts;
use outscale_api::apis::image_api::read_images;
use crate::core::{self, Resources};
use crate::debug;
use crate::VERSION;

static THROTTLING_MIN_WAIT_MS: u64 = 1000;
static THROTTLING_MAX_WAIT_MS: u64 = 10000;

type ImageId = String;
type VmId = String;
// This string correspond to pure internal forged identifier of a catalog entry.
// format!("{}:{}", entry.service, entry.type);
type CatalogId = String;

pub struct Input {
    config: Configuration,
    rng: ThreadRng,
    pub vms: HashMap::<VmId, Vm>,
    pub vms_images: HashMap<ImageId, Image>,
    pub catalog: HashMap<CatalogId, CatalogEntry>,
    pub account: Option<Account>,
    pub region: Option<String>,
    pub fetch_date: Option<DateTime<Utc>>,
}

impl Input {
    pub fn new(profile_name: String) -> Result<Input, Box<dyn error::Error>> {
        let config_file = ConfigurationFile::load_default()?;
        Ok(Input {
            config: config_file.configuration(profile_name)?,
            rng: thread_rng(),
            vms: HashMap::<VmId, Vm>::new(),
            vms_images: HashMap::<ImageId, Image>::new(),
            catalog: HashMap::<CatalogId, CatalogEntry>::new(),
            account: None,
            region: None,
            fetch_date: None,
        })
    }

    pub fn fetch(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.fetch_date = Some(Utc::now());
        self.fetch_vms()?;
        self.fetch_vms_images()?;
        self.fetch_catalog()?;
        self.fetch_account()?;
        self.fetch_region()?;
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

        let catalog = match catalog.entries {
            Some(entries) => entries,
            None => {
                if debug() {
                    eprintln!("warning: no catalog entries provided");
                }
                return Ok(());
            }
        };
        for entry in catalog {
            let _type = match &entry._type {
                Some(t) => t.clone(),
                None => {
                    if debug() {
                        eprintln!("warning: catalog entry as no type");
                    }
                    continue;
                }
            };
            let service = match &entry.service {
                Some(t) => t.clone(),
                None => {
                    if debug() {
                        eprintln!("warning: catalog entry as no service");
                    }
                    continue;
                }
            };
            let id = format!("{}:{}", service, _type);
            self.catalog.insert(id, entry);
        }

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

    fn fetch_region(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadSubregionsResponse = loop {
            let request = ReadSubregionsRequest::new();
            let response = read_subregions(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let subregions = match result.subregions {
            None => {
                if debug() {
                    eprintln!("warning: no region available");
                }
                return Ok(());
            },
            Some(subregions) => subregions,
        };
        self.region = match subregions.iter().next() {
            Some(subregion) => subregion.region_name.clone(),
            None => {
                if debug() {
                    eprintln!("warning: no subregion in region list");
                }
                return Ok(());
            }
        };
        if debug() {
            eprintln!("info: fetched region details");
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

    fn account_id(&self) -> Option<String> {
        match &self.account {
            Some(account) => match &account.account_id {
                Some(account_id) => Some(account_id.clone()),
                None => None,
            },
            None => None,
        }
    }

    fn fetch_date_rfc3339(&self) -> Option<String> {
        match self.fetch_date {
            Some(date) => Some(date.to_rfc3339()),
            None => None,
        }
    }

    fn vm_cpu_catalog_entry(&self, vm: &Vm) -> Option<CatalogEntry> {
        let vm_type = match &vm.vm_type {
            Some(vm_type) => vm_type,
            None => {
                if debug() {
                    eprintln!("warning: cannot get vm type in vm details");
                }
                return None;
            }
        };
        let id = match vm_type.starts_with("tina") {
            true => {
                // format: tinav5.c2r4p1
                //         0123456789012
                let gen = match vm_type.chars().nth(5) {
                    Some(c) => c,
                    None => {
                        if debug() {
                            eprintln!("warning: cannot parse generation for tina type {}", vm_type);
                        }
                        return None;
                    },
                };
                let perf = match vm_type.chars().nth(12) {
                    Some(c) => c,
                    None => {
                        if debug() {
                            eprintln!("warning: cannot parse performance for tina type {}", vm_type);
                        }
                        return None;
                    },
                };
                format!("TinaOS-FCU:CustomCore:v{}-p{}", gen, perf)
            },
            false => format!("TinaOS-FCU:BoxUsage:{}", vm_type),
        };
        match self.catalog.get(&id) {
            Some(entry) => Some(entry.clone()),
            None => {
                if debug() {
                    eprintln!("warning: cannot find cpu catalog entry for {}", vm_type);
                }
                None
            }
        }
    }
    
    fn vm_price_vcpu_per_hour(&self, vm: &Vm) -> f32 {
        match self.vm_cpu_catalog_entry(vm) {
            Some(entry) => match entry.unit_price {
                Some(price) => return price,
                None => {
                    if debug() {
                        eprintln!("warning: entry price is not defined");
                    }
                    0.0
                },
            },
            None => 0.0
        }
    }

    fn fill_resource_vm(&self, resources: &mut Resources) {
        for (vm_id, vm) in &self.vms {
            let core_vm = core::Vm {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                resource_type: Some(String::from("vm")),
                read_date_rfc3339: self.fetch_date_rfc3339(),
                region: self.region.clone(),
                resource_id: Some(vm_id.clone()),
                currency: None,
                price_per_hour: None,
                price_per_month: None,
                vm_type: vm.vm_type.clone(),
                vm_vcpu_gen: None,
                vm_core_performance: vm.performance.clone(),
                vm_image: vm.image_id.clone(),
                vm_product_id: None,
                vm_vcpu: 0,
                vm_ram_gb: 0,
                price_vcpu_per_hour: self.vm_price_vcpu_per_hour(&vm),
                price_ram_gb_per_hour: 0_f32,
                price_product_per_cpu_per_hour: 0_f32,
            };
            resources.vms.push(core_vm);
        }
    }
}

impl From<Input> for core::Resources {
    fn from(input: Input) -> Self {
        let mut resources = core::Resources {
            vms: Vec::new(),
        };
        input.fill_resource_vm(&mut resources);
        return resources;
    }
}
