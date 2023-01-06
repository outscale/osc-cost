use crate::args::Filter;
use crate::core::{self, Resources};
use crate::VERSION;
use chrono::{DateTime, Utc};
use http::status::StatusCode;
use lazy_static::lazy_static;
use log::{info, trace, warn};
use outscale_api::apis::account_api::read_accounts;
use outscale_api::apis::catalog_api::read_catalog;
use outscale_api::apis::configuration::AWSv4Key;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::image_api::read_images;
use outscale_api::apis::nat_service_api::read_nat_services;
use outscale_api::apis::public_ip_api::read_public_ips;
use outscale_api::apis::snapshot_api::read_snapshots;
use outscale_api::apis::subregion_api::read_subregions;
use outscale_api::apis::vm_api::{read_vm_types, read_vms};
use outscale_api::apis::volume_api::read_volumes;
use outscale_api::apis::Error::ResponseError;
use outscale_api::models::{
    Account, CatalogEntry, FiltersImage, FiltersNatService, FiltersPublicIp, FiltersSnapshot,
    FiltersVm, FiltersVolume, Image, NatService, PublicIp, ReadAccountsRequest,
    ReadAccountsResponse, ReadCatalogRequest, ReadCatalogResponse, ReadImagesRequest,
    ReadImagesResponse, ReadNatServicesRequest, ReadNatServicesResponse, ReadPublicIpsRequest,
    ReadPublicIpsResponse, ReadSnapshotsRequest, ReadSnapshotsResponse, ReadSubregionsRequest,
    ReadSubregionsResponse, ReadVmTypesRequest, ReadVmTypesResponse, ReadVmsRequest,
    ReadVmsResponse, ReadVolumesRequest, ReadVolumesResponse, Snapshot, Vm, VmType, Volume,
};
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};
use regex::Regex;
use secrecy::SecretString;
use std::collections::HashMap;
use std::convert::From;
use std::env;
use std::error;
use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

static THROTTLING_MIN_WAIT_MS: u64 = 1000;
static THROTTLING_MAX_WAIT_MS: u64 = 10000;

type ImageId = String;
type VmId = String;
type VolumeId = String;
type SnapshotId = String;
type NatServiceId = String;

// This string correspond to pure internal forged identifier of a catalog entry.
// format!("{}/{}/{}", entry.service, entry.type, entry.operation);
// use catalog_entry() to ease process
type CatalogId = String;
type VmTypeName = String;
type PublicIpId = String;

pub struct Input {
    config: Configuration,
    rng: ThreadRng,
    pub vms: HashMap<VmId, Vm>,
    pub vms_images: HashMap<ImageId, Image>,
    pub catalog: HashMap<CatalogId, CatalogEntry>,
    pub need_vm_types_fetch: bool,
    pub vm_types: HashMap<VmTypeName, VmType>,
    pub account: Option<Account>,
    pub region: Option<String>,
    pub nat_services: HashMap<NatServiceId, NatService>,
    pub volumes: HashMap<VolumeId, Volume>,
    pub snapshots: HashMap<SnapshotId, Snapshot>,
    pub fetch_date: Option<DateTime<Utc>>,
    pub public_ips: HashMap<PublicIpId, PublicIp>,
    pub filters: Option<Filter>,
}

impl Input {
    pub fn new(profile_name: String) -> Result<Input, Box<dyn error::Error>> {
        let config = Input::get_config(profile_name)?;
        Ok(Input {
            config,
            rng: thread_rng(),
            vms: HashMap::new(),
            vms_images: HashMap::new(),
            catalog: HashMap::new(),
            need_vm_types_fetch: false,
            vm_types: HashMap::new(),
            account: None,
            region: None,
            volumes: HashMap::<VolumeId, Volume>::new(),
            snapshots: HashMap::<SnapshotId, Snapshot>::new(),
            nat_services: HashMap::<NatServiceId, NatService>::new(),
            fetch_date: None,
            public_ips: HashMap::new(),
            filters: None,
        })
    }

    fn get_config(profile_name: String) -> Result<Configuration, Box<dyn Error>> {
        trace!("try to load api config from environment variables");
        let ak_env = env::var("OSC_ACCESS_KEY").ok();
        let sk_env = env::var("OSC_SECRET_KEY").ok();
        let region_env = env::var("OSC_REGION").ok();
        match (ak_env, sk_env, region_env) {
            (Some(access_key), Some(secret_key), Some(region)) => {
                let mut config = Configuration::new();
                config.base_path = format!("https://api.{}.outscale.com/api/v1", region);
                config.aws_v4_key = Some(AWSv4Key {
                    region,
                    access_key,
                    secret_key: SecretString::new(secret_key),
                    service: "oapi".to_string(),
                });
                config.user_agent = Some(format!("osc-cost/{VERSION}"));
                return Ok(config);
            }
            (None, None, None) => {}
            (_, _, _) => {
                warn!("some credentials are set through environement variable but not all. OSC_ACCESS_KEY, OSC_SECRET_KEY and OSC_REGION are required to use this method.");
            }
        };

        trace!("try to load api config from configuration file");
        let config_file = ConfigurationFile::load_default()?;
        let mut config = config_file.configuration(profile_name)?;
        config.user_agent = Some(format!("osc-cost/{VERSION}"));
        Ok(config)
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
        self.fetch_volumes()?;
        self.fetch_nat_services()?;
        self.fetch_public_ips()?;
        self.fetch_snapshots()?;
        Ok(())
    }

    fn fetch_vms(&mut self) -> Result<(), Box<dyn error::Error>> {
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
                warn!("no catalog provided");
                return Ok(());
            }
        };

        let catalog = match catalog.entries {
            Some(entries) => entries,
            None => {
                warn!("no catalog entries provided");
                return Ok(());
            }
        };
        for entry in catalog {
            let _type = match &entry._type {
                Some(t) => t.clone(),
                None => {
                    warn!("catalog entry as no type");
                    continue;
                }
            };
            let service = match &entry.service {
                Some(t) => t.clone(),
                None => {
                    warn!("catalog entry as no service");
                    continue;
                }
            };
            let operation = match &entry.operation {
                Some(t) => t.clone(),
                None => {
                    warn!("catalog entry as no operation");
                    continue;
                }
            };
            let entry_id = format!("{}/{}/{}", service, _type, operation);
            self.catalog.insert(entry_id, entry);
        }

        info!("fetched {} catalog entries", self.catalog.len());
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
                warn!("no account available");
                return Ok(());
            }
            Some(accounts) => accounts,
        };
        self.account = match accounts.first() {
            Some(account) => Some(account.clone()),
            None => {
                warn!("no account in account list");
                return Ok(());
            }
        };
        info!("fetched account details");
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
                warn!("no region available");
                return Ok(());
            }
            Some(subregions) => subregions,
        };
        self.region = match subregions.first() {
            Some(subregion) => subregion.region_name.clone(),
            None => {
                warn!("no subregion in region list");
                return Ok(());
            }
        };
        info!("fetched region details");
        Ok(())
    }

    fn fetch_volumes(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadVolumesResponse = loop {
            let filter_volumes: FiltersVolume = match &self.filters {
                Some(filter) => FiltersVolume {
                    tag_keys: Some(filter.filter_tag_key.clone()),
                    tag_values: Some(filter.filter_tag_value.clone()),
                    tags: Some(filter.filter_tag.clone()),
                    ..Default::default()
                },
                None => FiltersVolume::new(),
            };
            let request = ReadVolumesRequest {
                filters: Some(Box::new(filter_volumes)),
                ..Default::default()
            };
            let response = read_volumes(&self.config, Some(request));

            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let volumes = match result.volumes {
            None => {
                warn!("no volume available");
                return Ok(());
            }
            Some(volumes) => volumes,
        };
        for volume in volumes {
            let volume_id = volume.volume_id.clone().unwrap_or_else(|| String::from(""));
            self.volumes.insert(volume_id, volume);
        }
        info!("fetched {} volumes", self.volumes.len());
        Ok(())
    }

    fn fetch_nat_services(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadNatServicesResponse = loop {
            let filters: FiltersNatService = match &self.filters {
                Some(filter) => FiltersNatService {
                    tag_keys: Some(filter.filter_tag_key.clone()),
                    tag_values: Some(filter.filter_tag_value.clone()),
                    tags: Some(filter.filter_tag.clone()),
                    ..Default::default()
                },
                None => FiltersNatService::new(),
            };
            let request = ReadNatServicesRequest {
                filters: Some(Box::new(filters)),
                ..Default::default()
            };
            let response = read_nat_services(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };
        let nat_services = match result.nat_services {
            None => {
                warn!("no nat_service available!");
                return Ok(());
            }
            Some(nat_services) => nat_services,
        };

        for nat_service in nat_services {
            let nat_service_id = nat_service
                .nat_service_id
                .clone()
                .unwrap_or_else(|| String::from(""));
            self.nat_services.insert(nat_service_id, nat_service);
        }
        info!("fetched {} nat_service", self.nat_services.len());
        Ok(())
    }
    fn fetch_public_ips(&mut self) -> Result<(), Box<dyn error::Error>> {
        let filters: FiltersPublicIp = match &self.filters {
            Some(filter) => FiltersPublicIp {
                tag_keys: Some(filter.filter_tag_key.clone()),
                tag_values: Some(filter.filter_tag_value.clone()),
                tags: Some(filter.filter_tag.clone()),
                ..Default::default()
            },
            None => FiltersPublicIp::new(),
        };
        let request = ReadPublicIpsRequest {
            filters: Some(Box::new(filters)),
            ..Default::default()
        };
        let result: ReadPublicIpsResponse = loop {
            let response = read_public_ips(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };
        let public_ips = match result.public_ips {
            None => {
                warn!("no public ip list provided");
                return Ok(());
            }
            Some(ips) => ips,
        };
        for public_ip in public_ips {
            let public_ip_id = match &public_ip.public_ip_id {
                Some(id) => id.clone(),
                None => {
                    warn!("public ip has no id");
                    continue;
                }
            };
            self.public_ips.insert(public_ip_id, public_ip);
        }

        info!("info: fetched {} public ips", self.public_ips.len());
        Ok(())
    }

    fn fetch_snapshots(&mut self) -> Result<(), Box<dyn error::Error>> {
        let account_id = match self.account_id() {
            None => {
                warn!("warning: no account_id available... skipping");
                return Ok(());
            }
            Some(account_id) => account_id,
        };
        let filters: FiltersSnapshot = match &self.filters {
            Some(filter) => FiltersSnapshot {
                account_ids: Some(vec![account_id]),
                tag_keys: Some(filter.filter_tag_key.clone()),
                tag_values: Some(filter.filter_tag_value.clone()),
                tags: Some(filter.filter_tag.clone()),
                ..Default::default()
            },
            None => FiltersSnapshot {
                account_ids: Some(vec![account_id]),
                ..Default::default()
            },
        };
        let request = ReadSnapshotsRequest {
            filters: Some(Box::new(filters)),
            ..Default::default()
        };
        let result: ReadSnapshotsResponse = loop {
            let response = read_snapshots(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let snapshots = match result.snapshots {
            None => {
                warn!("warning: no snapshot available");
                return Ok(());
            }
            Some(snapshots) => snapshots,
        };
        for snapshot in snapshots {
            let snapshot_id = snapshot
                .snapshot_id
                .clone()
                .unwrap_or_else(|| String::from(""));
            self.snapshots.insert(snapshot_id, snapshot);
        }
        warn!("info: fetched {} snapshots", self.snapshots.len());
        Ok(())
    }

    fn random_wait(&mut self) {
        let wait_time_ms = self
            .rng
            .gen_range(THROTTLING_MIN_WAIT_MS..THROTTLING_MAX_WAIT_MS);
        info!("call throttled, waiting for {}ms", wait_time_ms);
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

    fn catalog_entry<S: Into<String>>(&self, service: S, type_: S, operation: S) -> Option<f32> {
        let entry_id = format!("{}/{}/{}", service.into(), type_.into(), operation.into());
        match self.catalog.get(&entry_id) {
            Some(entry) => match entry.unit_price {
                Some(price) => Some(price),
                None => {
                    warn!("cannot find price for {}", entry_id);
                    None
                }
            },
            None => {
                warn!("cannot find catalog entry for {}", entry_id);
                None
            }
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
            resources.resources.push(core::Resource::Vm(core_vm));
        }
    }

    fn fill_resource_volume(&self, resources: &mut Resources) {
        for (volume_id, volume) in &self.volumes {
            let specs = match VolumeSpecs::new(volume, self) {
                Some(s) => s,
                None => continue,
            };
            let core_volume = core::Volume {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(volume_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                volume_type: Some(specs.volume_type.clone()),
                volume_iops: Some(specs.iops),
                volume_size: Some(specs.size),
                price_gb_per_month: specs.price_gb_per_month,
                price_iops_per_month: specs.price_iops_per_month,
            };
            resources
                .resources
                .push(core::Resource::Volume(core_volume));
        }
    }
    fn fill_resource_nat_service(&self, resources: &mut Resources) {
        for (nat_service_id, nat_service) in &self.nat_services {
            let price_product_per_nat_service_per_hour =
                self.catalog_entry("TinaOS-FCU", "NatGatewayUsage", "CreateNatGateway");
            let Some(nat_service_id) = &nat_service.nat_service_id else {
                warn!("cannot get nat_service_id content for {}", nat_service_id );
                continue;
            };
            let core_nat_service = core::NatServices {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(nat_service_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                price_product_per_nat_service_per_hour,
            };
            resources
                .resources
                .push(core::Resource::NatServices(core_nat_service));
        }
    }

    fn fill_resource_public_ip(&self, resources: &mut Resources) {
        for (public_ip_id, public_ip) in &self.public_ips {
            let mut price_non_attached: Option<f32> = None;
            let mut price_first_ip: Option<f32> = None;
            let mut price_next_ips: Option<f32> = None;
            let Some(public_ip_str) = &public_ip.public_ip else {
                warn!("cannot get public ip content for {}", public_ip_id);
                continue;
            };

            match &public_ip.vm_id {
                None => match self.catalog_entry(
                    "TinaOS-FCU",
                    "ElasticIP:IdleAddress",
                    "AssociateAddressVPC",
                ) {
                    Some(price) => price_non_attached = Some(price),
                    None => continue,
                },
                Some(vm_id) => match self.vms.get(vm_id) {
                    Some(vm) => match &vm.public_ip {
                        Some(vm_public_ip) => match *vm_public_ip == *public_ip_str {
                            // First Public IP is free
                            true => price_first_ip = Some(0_f32),
                            // Additional Public IP cost
                            false => {
                                price_next_ips = match self.catalog_entry(
                                    "TinaOS-FCU",
                                    "ElasticIP:AdditionalAddress",
                                    "AssociateAddressVPC",
                                ) {
                                    Some(price) => Some(price),
                                    None => continue,
                                }
                            }
                        },
                        None => {
                            warn!(
                                "vm {} does not seem to have any ip, should at least have {}",
                                vm_id, public_ip_str
                            );
                            continue;
                        }
                    },
                    None => {
                        warn!(
                            "cannot find vm id {} for public ip {} ({})",
                            vm_id, public_ip_str, public_ip_id
                        );
                        continue;
                    }
                },
            };
            let core_public_ip = core::PublicIp {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(public_ip_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                price_non_attached,
                price_first_ip,
                price_next_ips,
            };
            resources
                .resources
                .push(core::Resource::PublicIp(core_public_ip));
        }
    }

    fn fill_resource_snapshot(&self, resources: &mut Resources) {
        let Some(price_gb_per_month) = self.catalog_entry("TinaOS-FCU", "Snapshot:Usage", "Snapshot") else {
            warn!("gib price is not defined for snapshot");
            return
        };
        for (snapshot_id, snapshot) in &self.snapshots {
            let core_snapshot = core::Snapshot {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(snapshot_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                volume_size_gib: snapshot.volume_size,
                price_gb_per_month,
            };
            resources
                .resources
                .push(core::Resource::Snapshot(core_snapshot));
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

struct VolumeSpecs {
    volume_type: String,
    size: i32,
    iops: i32,
    price_gb_per_month: f32,
    price_iops_per_month: f32,
}

impl VolumeSpecs {
    fn new(volume: &Volume, input: &Input) -> Option<Self> {
        let volume_type = match &volume.volume_type {
            Some(volume_type) => volume_type,
            None => {
                warn!("warning: cannot get volume type in volume details");
                return None;
            }
        };

        let iops = volume.iops.unwrap_or_else(|| {
            if volume_type == "io1" {
                warn!("cannot get iops in volume details");
            }
            0
        });

        let size = match &volume.size {
            Some(size) => *size,
            None => {
                warn!("cannot get size in volume details");
                return None;
            }
        };
        let out = VolumeSpecs {
            volume_type: volume_type.clone(),
            iops,
            size,
            price_gb_per_month: 0_f32,
            price_iops_per_month: 0_f32,
        };

        out.parse_volume_prices(input)
    }

    fn parse_volume_prices(mut self, input: &Input) -> Option<VolumeSpecs> {
        let price_gb_per_month = input.catalog_entry(
            "TinaOS-FCU",
            &format!("BSU:VolumeUsage:{}", self.volume_type),
            "CreateVolume",
        )?;
        if self.volume_type == "io1" {
            self.price_iops_per_month = input.catalog_entry(
                "TinaOS-FCU",
                &format!("BSU:VolumeIOPS:{}", self.volume_type),
                "CreateVolume",
            )?;
        }
        self.price_gb_per_month = price_gb_per_month;
        Some(self)
    }
}

impl From<Input> for core::Resources {
    fn from(input: Input) -> Self {
        let mut resources = core::Resources {
            resources: Vec::new(),
        };
        input.fill_resource_vm(&mut resources);
        input.fill_resource_volume(&mut resources);
        input.fill_resource_public_ip(&mut resources);
        input.fill_resource_snapshot(&mut resources);
        input.fill_resource_nat_service(&mut resources);
        resources
    }
}
