use crate::core::{self, Resources};
use crate::debug;
use crate::VERSION;
use chrono::{DateTime, Utc};
use http::status::StatusCode;
use lazy_static::lazy_static;
use outscale_api::apis::account_api::read_accounts;
use outscale_api::apis::catalog_api::read_catalog;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::image_api::read_images;
use outscale_api::apis::subregion_api::read_subregions;
use outscale_api::apis::vm_api::{read_vm_types, read_vms};
use outscale_api::apis::Error::ResponseError;
use outscale_api::models::{
    Account, CatalogEntry, FiltersImage, Image, ReadAccountsRequest, ReadAccountsResponse,
    ReadCatalogRequest, ReadCatalogResponse, ReadImagesRequest, ReadImagesResponse,
    ReadSubregionsRequest, ReadSubregionsResponse, ReadVmTypesRequest, ReadVmTypesResponse,
    ReadVmsRequest, ReadVmsResponse, Vm, VmType,
};
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};
use regex::Regex;
use std::collections::HashMap;
use std::convert::From;
use std::error;
use std::thread::sleep;
use std::time::Duration;

static THROTTLING_MIN_WAIT_MS: u64 = 1000;
static THROTTLING_MAX_WAIT_MS: u64 = 10000;

type ImageId = String;
type VmId = String;
// This string correspond to pure internal forged identifier of a catalog entry.
// format!("{}/{}/{}", entry.service, entry.type, entry.operation);
type CatalogId = String;
type VmTypeName = String;

pub struct Input {
    config: Configuration,
    rng: ThreadRng,
    pub vms: HashMap<VmId, Vm>,
    pub vms_images: HashMap<ImageId, Image>,
    pub catalog: HashMap<CatalogId, CatalogEntry>,
    pub need_vm_types_fetch: bool,
    pub vm_types: HashMap<String, VmType>,
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
            need_vm_types_fetch: false,
            vm_types: HashMap::<VmTypeName, VmType>::new(),
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
        if self.need_vm_types_fetch {
            self.fetch_vm_types()?;
        }
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
            }
            Some(vms) => vms,
        };
        for vm in vms {
            if let Some(state) = &vm.state {
                match state.as_str() {
                    "running" | "stopping" | "shutting-down" => {}
                    "pending" | "stopped" | "terminated" | "quarantine" => continue,
                    _ => {
                        if debug() {
                            eprintln!("warning: un-managed vm state {} found", state);
                        }
                        continue;
                    }
                };
            }
            let vm_id = match &vm.vm_id {
                Some(id) => id,
                None => {
                    if debug() {
                        eprintln!("warning: vm has no id");
                    }
                    continue;
                }
            };
            if let Some(vm_type) = &vm.vm_type {
                if !self.need_vm_types_fetch && !vm_type.starts_with("tina") {
                    self.need_vm_types_fetch = true;
                }
            }
            self.vms.insert(vm_id.clone(), vm);
        }

        if debug() {
            eprintln!("info: fetched {} vms", self.vms.len());
        }
        Ok(())
    }

    fn fetch_vms_images(&mut self) -> Result<(), Box<dyn error::Error>> {
        // Collect all unique images
        let mut images = Vec::<ImageId>::new();
        for vm_id in self.vms.keys() {
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
            }
            Some(images) => images,
        };
        for image in images {
            let image_id = image.image_id.clone().unwrap_or_else(|| String::from(""));
            self.vms_images.insert(image_id, image);
        }

        if debug() {
            eprintln!("info: fetched {} images used by vms", self.vms_images.len());
        }
        Ok(())
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
            }
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
            let operation = match &entry.operation {
                Some(t) => t.clone(),
                None => {
                    if debug() {
                        eprintln!("warning: catalog entry as no operation");
                    }
                    continue;
                }
            };
            let entry_id = format!("{}/{}/{}", service, _type, operation);
            self.catalog.insert(entry_id, entry);
        }

        if debug() {
            eprintln!("info: fetched {} catalog entries", self.catalog.len());
        }
        Ok(())
    }

    fn fetch_vm_types(&mut self) -> Result<(), Box<dyn error::Error>> {
        let request = ReadVmTypesRequest::new();
        let result: ReadVmTypesResponse = loop {
            let response = read_vm_types(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };
        let vm_types = match result.vm_types {
            None => {
                if debug() {
                    eprintln!("warning: no vm type list provided");
                }
                return Ok(());
            }
            Some(vm_types) => vm_types,
        };
        for vm_type in vm_types {
            let vm_type_name = match &vm_type.vm_type_name {
                Some(name) => name.clone(),
                None => {
                    if debug() {
                        eprintln!("warning: vm type has no name");
                    }
                    continue;
                }
            };
            self.vm_types.insert(vm_type_name, vm_type);
        }

        if debug() {
            eprintln!("info: fetched {} vm types", self.vm_types.len());
        }
        Ok(())
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
            }
            Some(accounts) => accounts,
        };
        self.account = match accounts.first() {
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
        Ok(())
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
            }
            Some(subregions) => subregions,
        };
        self.region = match subregions.first() {
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
        Ok(())
    }

    fn random_wait(&mut self) {
        let wait_time_ms = self
            .rng
            .gen_range(THROTTLING_MIN_WAIT_MS..THROTTLING_MAX_WAIT_MS);
        if debug() {
            eprintln!("info: call throttled, waiting for {}ms", wait_time_ms);
        }
        sleep(Duration::from_millis(wait_time_ms));
    }

    fn is_throttled<S, T>(result: &Result<S, outscale_api::apis::Error<T>>) -> bool {
        match result {
            Ok(_) => false,
            Err(error) => match error {
                ResponseError(resp) => matches!(
                    resp.status,
                    StatusCode::SERVICE_UNAVAILABLE | StatusCode::TOO_MANY_REQUESTS
                ),
                _ => false,
            },
        }
    }

    fn account_id(&self) -> Option<String> {
        match &self.account {
            Some(account) => account.account_id.clone(),
            None => None,
        }
    }

    fn fill_resource_vm(&self, resources: &mut Resources) {
        for (vm_id, vm) in &self.vms {
            let specs = match VmSpecs::new(vm, self) {
                Some(s) => s,
                None => continue,
            };
            let core_vm = core::Vm {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                resource_type: Some(String::from("vm")),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(vm_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                vm_type: vm.vm_type.clone(),
                vm_vcpu_gen: Some(specs.generation.clone()),
                vm_core_performance: vm.performance.clone(),
                vm_image: vm.image_id.clone(),
                vm_vcpu: specs.vcpu,
                vm_ram_gb: specs.ram_gb,
                price_vcpu_per_hour: specs.price_vcpu_per_hour,
                price_ram_gb_per_hour: specs.price_ram_gb_per_hour,
                // Mandatory to compute price for BoxUsage (aws-type, etc) types
                price_box_per_hour: specs.price_box_per_hour,
                // Mandatory to compute price for all vm types
                price_product_per_ram_gb_per_hour: specs.price_product_per_ram_gb_per_hour,
                price_product_per_cpu_per_hour: specs.price_product_per_cpu_per_hour,
                price_product_per_vm_per_hour: specs.price_product_per_vm_per_hour,
            };
            resources.vms.push(core_vm);
        }
    }
}

struct VmSpecs {
    vm_type: String,
    generation: String,
    vcpu: usize,
    ram_gb: usize,
    performance: String,
    product_codes: Vec<String>,
    price_vcpu_per_hour: f32,
    price_ram_gb_per_hour: f32,
    price_box_per_hour: f32,
    price_product_per_ram_gb_per_hour: f32,
    price_product_per_cpu_per_hour: f32,
    price_product_per_vm_per_hour: f32,
}

impl VmSpecs {
    fn new(vm: &Vm, input: &Input) -> Option<Self> {
        let vm_type = match &vm.vm_type {
            Some(vm_type) => vm_type,
            None => {
                if debug() {
                    eprintln!("warning: cannot get vm type in vm details");
                }
                return None;
            }
        };
        let out = VmSpecs {
            vm_type: vm_type.clone(),
            generation: String::from(""),
            vcpu: 0,
            ram_gb: 0,
            performance: String::from(""),
            product_codes: vm.product_codes.clone().unwrap_or_default(),
            price_vcpu_per_hour: 0_f32,
            price_ram_gb_per_hour: 0_f32,
            price_box_per_hour: 0_f32,
            price_product_per_ram_gb_per_hour: 0_f32,
            price_product_per_cpu_per_hour: 0_f32,
            price_product_per_vm_per_hour: 0_f32,
        };
        match vm_type.starts_with("tina") {
            true => out
                .parse_tina_type()?
                .parse_product_price(input)?
                .parse_tina_prices(input),
            false => out
                .parse_box_type(input)?
                .parse_product_price(input)?
                .parse_box_prices(input),
        }
    }

    fn parse_tina_type(mut self) -> Option<VmSpecs> {
        // format: tinav5.c20r40p1
        //              │  ││ ││ │
        //              │  ││ ││ └── vcpu performance
        //              │  ││ └┴── ram quantity
        //              │  └┴── number of vcores
        //              └── generation
        lazy_static! {
            static ref REG: Regex = Regex::new(r"^tinav(\d+)\.c(\d+)r(\d+)p(\d+)$").unwrap();
        }
        let cap = REG.captures_iter(&self.vm_type).next()?;
        self.generation = String::from(&cap[1]);
        self.vcpu = cap[2].parse::<usize>().ok()?;
        self.ram_gb = cap[3].parse::<usize>().ok()?;
        self.performance = String::from(&cap[4]);
        Some(self)
    }

    fn parse_product_price(mut self, input: &Input) -> Option<VmSpecs> {
        for product_code in &self.product_codes {
            let entry_id = format!("TinaOS-FCU/ProductUsage/RunInstances-{}-OD", product_code);
            let price = match input.catalog.get(&entry_id) {
                Some(entry) => match entry.unit_price {
                    Some(price) => price,
                    None => {
                        if debug() {
                            eprintln!(
                                "warning: product price is not defined for product code {}",
                                product_code
                            );
                        }
                        continue;
                    }
                },
                None => {
                    if debug() {
                        eprintln!("warning: cannot find product code {}", product_code);
                    }
                    continue;
                }
            };
            // License calculation is specific to each product code.
            // https://en.outscale.com/pricing/#licenses
            let cores = self.vcpu as f32;
            match product_code.as_str() {
                // Generic Linux vm, should be free
                "0001" => {}
                // Microsoft Windows Server 2019 License
                // Price by 2 cores
                "0002" => {
                    let cores_to_pay = ((cores + 1.0) / 2.0).floor();
                    let price_for_vm = cores_to_pay * price;
                    // set back price per cpu per hour
                    self.price_product_per_cpu_per_hour += price_for_vm / cores;
                }
                // mapr license (0003)
                // Oracle Linux OS Distribution (0004)
                // Microsoft Windows 10 E3 VDA License (0005)
                // Red Hat Enterprise Linux OS Distribution (0006)
                // sql server web (0007)
                "0003" | "0004" | "0005" | "0006" | "0007" => {
                    self.price_product_per_vm_per_hour += price
                }
                // Microsoft Windows SQL Server 2019 Standard Edition (0008)
                // Microsoft Windows SQL Server 2019 Enterprise Edition (0009)
                // Price by 2 cores (4 cores min)
                "0008" | "0009" => {
                    let cores_to_pay = ((cores + 1.0) / 2.0).floor().max(4.0);
                    let price_for_vm = cores_to_pay * price;
                    // set back price per cpu per hour
                    self.price_product_per_cpu_per_hour += price_for_vm / cores;
                }
                _ => {
                    if debug() {
                        eprintln!("warning: product code {} is not managed", product_code);
                    }
                    continue;
                }
            };
        }
        Some(self)
    }

    fn parse_tina_prices(mut self, input: &Input) -> Option<VmSpecs> {
        let entry_id = format!(
            "TinaOS-FCU/CustomCore:v{}-p{}/RunInstances-OD",
            self.generation, self.performance
        );
        self.price_vcpu_per_hour = match input.catalog.get(&entry_id) {
            Some(entry) => match entry.unit_price {
                Some(price) => price,
                None => {
                    if debug() {
                        eprintln!(
                            "warning: cpu price is not defined for VM type tina {}",
                            self.vm_type
                        );
                    }
                    return None;
                }
            },
            None => {
                if debug() {
                    eprintln!(
                        "warning: cannot find cpu catalog entry for VM type tina {}",
                        self.vm_type
                    );
                }
                return None;
            }
        };

        let entry_id = "TinaOS-FCU/CustomRam/RunInstances-OD".to_string();
        self.price_ram_gb_per_hour = match input.catalog.get(&entry_id) {
            Some(entry) => match entry.unit_price {
                Some(price) => price,
                None => {
                    if debug() {
                        eprintln!(
                            "warning: ram price is not defined for VM type tina {}",
                            self.vm_type
                        );
                    }
                    return None;
                }
            },
            None => {
                if debug() {
                    eprintln!(
                        "warning: cannot find ram catalog entry for VM type tina {}",
                        self.vm_type
                    );
                }
                return None;
            }
        };
        Some(self)
    }

    fn parse_box_type(mut self, input: &Input) -> Option<VmSpecs> {
        let vm_type = input.vm_types.get(&self.vm_type)?;
        self.vcpu = vm_type.vcore_count? as usize;
        self.ram_gb = vm_type.memory_size? as usize;
        // We don't have this information through API for now but we have it on public documentation:
        // https://docs.outscale.com/en/userguide/Instance-Types.html
        // format: m4.4xlarge
        //         └┴── vm family
        lazy_static! {
            static ref REG: Regex = Regex::new(r"^([a-z]+\d+)\..*$").unwrap();
        }
        let cap = match REG.captures_iter(&self.vm_type).next() {
            Some(cap) => cap,
            // family and generation is not mandatory to extract price.
            None => {
                if debug() {
                    eprintln!("warning: annot extract vm family from {}", self.vm_type);
                }
                return Some(self);
            }
        };
        let family = String::from(&cap[1]);
        let (generation, performance) = match family.as_str() {
            "c1" => ("1", "1"),
            "c3" => ("3", "1"),
            "c4" => ("4", "1"),
            "c5" => ("5", "1"),
            "cc1" => ("1", "1"),
            "cc2" => ("2", "2"),
            "cr1" => ("1", "2"),
            "g2" => ("2", "1"),
            "g3" => ("3", "1"),
            "hi1" => ("3", "1"),
            "i2" => ("4", "2"),
            "io5" => ("4", "2"),
            "m1" => ("1", "2"),
            "m2" => ("2", "2"),
            "m3" => ("3", "2"),
            "m4" => ("4", "2"),
            "m5" => ("5", "2"),
            "mv3" => ("4", "2"),
            "nv1" => ("4", "2"),
            "nv2" => ("4", "2"),
            "oc1" => ("1", "1"),
            "oc2" => ("2", "1"),
            "oc5" => ("4", "1"),
            "og3" => ("4", "2"),
            "og4" => ("4", "1"),
            "om5" => ("4", "2"),
            "os1" => ("1", "2"),
            "os3" => ("3", "2"),
            "p100" => ("5", "1"),
            "p3" => ("5", "1"),
            "p6" => ("5", "1"),
            "r3" => ("3", "2"),
            "r4" => ("4", "2"),
            "t1" => ("1", "3"),
            "t2" => ("2", "3"),
            unknown_family => {
                if debug() {
                    eprintln!("warning: unkown family name for {}", unknown_family)
                }
                ("", "")
            }
        };
        self.generation = String::from(generation);
        self.performance = String::from(performance);
        Some(self)
    }

    fn parse_box_prices(mut self, input: &Input) -> Option<VmSpecs> {
        let entry_id = format!("TinaOS-FCU/BoxUsage:{}/RunInstances-OD", self.vm_type);
        self.price_box_per_hour = match input.catalog.get(&entry_id) {
            Some(entry) => match entry.unit_price {
                Some(price) => price,
                None => {
                    if debug() {
                        eprintln!(
                            "warning: cpu price is not defined for VM type BoxUsage {}",
                            self.vm_type
                        );
                    }
                    return None;
                }
            },
            None => {
                if debug() {
                    eprintln!(
                        "warning: cannot find cpu catalog entry for VM type BoxUsage {}",
                        self.vm_type
                    );
                }
                return None;
            }
        };
        Some(self)
    }
}

impl From<Input> for core::Resources {
    fn from(input: Input) -> Self {
        let mut resources = core::Resources { vms: Vec::new() };
        input.fill_resource_vm(&mut resources);
        resources
    }
}
