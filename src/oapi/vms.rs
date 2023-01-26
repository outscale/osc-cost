use std::error;

use log::{info, warn};
use outscale_api::{
    apis::{
        image_api::read_images,
        vm_api::{read_vm_types, read_vms},
    },
    models::{
        FiltersImage, FiltersVm, ReadImagesRequest, ReadImagesResponse, ReadVmTypesRequest,
        ReadVmTypesResponse, ReadVmsRequest, ReadVmsResponse,
    },
};

use crate::{
    core::{vms::Vm, Resource, Resources},
    oapi::ImageId,
    VERSION,
};
use lazy_static::lazy_static;
use regex::Regex;

use super::Input;

pub type VmId = String;

impl Input {
    pub fn fetch_vms(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadVmsResponse = loop {
            let filter_vm: FiltersVm = match &self.filters {
                Some(filter) => FiltersVm {
                    tag_keys: Some(filter.filter_tag_key.clone()),
                    tag_values: Some(filter.filter_tag_value.clone()),
                    tags: Some(filter.filter_tag.clone()),
                    ..Default::default()
                },
                None => FiltersVm::new(),
            };

            let request = ReadVmsRequest {
                filters: Some(Box::new(filter_vm)),
                ..Default::default()
            };
            let response = read_vms(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let vms = match result.vms {
            None => {
                warn!("no vm list provided");
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
                        warn!("un-managed vm state {} found", state);
                        continue;
                    }
                };
            }
            let vm_id = match &vm.vm_id {
                Some(id) => id,
                None => {
                    warn!("vm has no id");
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
        info!("fetched {} vms", self.vms.len());
        Ok(())
    }

    pub fn fetch_vms_images(&mut self) -> Result<(), Box<dyn error::Error>> {
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
                warn!("no image list provided");
                return Ok(());
            }
            Some(images) => images,
        };
        for image in images {
            let image_id = image.image_id.clone().unwrap_or_else(|| String::from(""));
            self.vms_images.insert(image_id, image);
        }
        info!("fetched {} images used by vms", self.vms_images.len());
        Ok(())
    }

    pub fn fetch_vm_types(&mut self) -> Result<(), Box<dyn error::Error>> {
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
                warn!("no vm type list provided");
                return Ok(());
            }
            Some(vm_types) => vm_types,
        };
        for vm_type in vm_types {
            let vm_type_name = match &vm_type.vm_type_name {
                Some(name) => name.clone(),
                None => {
                    warn!("vm type has no name");
                    continue;
                }
            };
            self.vm_types.insert(vm_type_name, vm_type);
        }

        info!("fetched {} vm types", self.vm_types.len());
        Ok(())
    }

    pub fn fill_resource_vm(&self, resources: &mut Resources) {
        for (vm_id, vm) in &self.vms {
            let specs = match VmSpecs::new(vm, self) {
                Some(s) => s,
                None => continue,
            };
            let core_vm = Vm {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
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
                price_license_per_ram_gb_per_hour: specs.price_product_per_ram_gb_per_hour,
                price_license_per_cpu_per_hour: specs.price_product_per_cpu_per_hour,
                price_license_per_vm_per_hour: specs.price_product_per_vm_per_hour,
            };
            resources.resources.push(Resource::Vm(core_vm));
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
    fn new(vm: &outscale_api::models::Vm, input: &Input) -> Option<Self> {
        let vm_type = match &vm.vm_type {
            Some(vm_type) => vm_type,
            None => {
                warn!("cannot get vm type in vm details");
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
            let Some(price) = input.catalog_entry("TinaOS-FCU", "ProductUsage", &format!("RunInstances-{}-OD", product_code)) else {
                continue;
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
                    warn!("product code {} is not managed", product_code);
                    continue;
                }
            };
        }
        Some(self)
    }

    fn parse_tina_prices(mut self, input: &Input) -> Option<VmSpecs> {
        let price_vcpu_per_hour = input.catalog_entry(
            "TinaOS-FCU",
            &format!("CustomCore:v{}-p{}", self.generation, self.performance),
            "RunInstances-OD",
        )?;
        let price_ram_gb_per_hour =
            input.catalog_entry("TinaOS-FCU", "CustomRam", "RunInstances-OD")?;
        self.price_vcpu_per_hour = price_vcpu_per_hour;
        self.price_ram_gb_per_hour = price_ram_gb_per_hour;
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
                warn!("annot extract vm family from {}", self.vm_type);
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
                warn!("unkown family name for {}", unknown_family);
                ("", "")
            }
        };
        self.generation = String::from(generation);
        self.performance = String::from(performance);
        Some(self)
    }

    fn parse_box_prices(mut self, input: &Input) -> Option<VmSpecs> {
        self.price_box_per_hour = input.catalog_entry(
            "TinaOS-FCU",
            &format!("BoxUsage:{}", self.vm_type),
            "RunInstances-OD",
        )?;
        Some(self)
    }
}
