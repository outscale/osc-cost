use crate::core::Resources;
use crate::VERSION;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_credential_types::Credentials;
use chrono::{DateTime, Utc};
use log::debug;
use log::{info, trace, warn};
use outscale_api::apis::account_api::read_accounts;
use outscale_api::apis::catalog_api::read_catalog;
use outscale_api::apis::configuration::AWSv4Key;
use outscale_api::apis::configuration::Configuration;
use outscale_api::apis::configuration_file::{ConfigurationFile, ConfigurationFileError};
use outscale_api::apis::subregion_api::read_subregions;
use outscale_api::models::ConsumptionEntry;
use outscale_api::models::{
    Account, CatalogEntry, FlexibleGpu, Image, NatService, PublicIp, ReadAccountsRequest,
    ReadAccountsResponse, ReadCatalogRequest, ReadCatalogResponse, ReadSubregionsRequest,
    ReadSubregionsResponse, Snapshot, Vm, VmType, Volume,
};
use std::collections::HashMap;
use std::convert::From;
use std::env;
use std::error;
use std::error::Error;

use self::flexible_gpus::FlexibleGpuId;
use self::load_balancers::LoadbalancerId;
use self::nat_services::NatServiceId;
use self::oos::{BucketId, OosBucket};
use self::public_ips::PublicIpId;
use self::snapshots::SnapshotId;
use self::vms::VmId;
use self::volumes::VolumeId;
use self::vpn::VpnId;

type ImageId = String;

// This string correspond to pure internal forged identifier of a catalog entry.
// format!("{}/{}/{}", entry.service, entry.type, entry.operation);
// use catalog_entry() to ease process
type CatalogId = String;
type VmTypeName = String;

mod dedicated_instances;
mod digest;
mod flexible_gpus;
mod load_balancers;
mod nat_services;
mod oos;
mod public_ips;
mod snapshots;
mod vms;
mod volumes;
mod vpn;

pub struct Filter {
    pub tag_keys: Vec<String>,
    pub tag_values: Vec<String>,
    pub tags: Vec<String>,
    pub skip_resource: Vec<String>,
}

pub struct Input {
    config: Configuration,
    aws_config: SdkConfig,
    pub vms: HashMap<VmId, Vm>,
    pub vms_images: HashMap<ImageId, Image>,
    pub catalog: HashMap<CatalogId, CatalogEntry>,
    pub need_vm_types_fetch: bool,
    pub use_dedicated_instance: bool,
    pub vm_types: HashMap<VmTypeName, VmType>,
    pub account: Option<Account>,
    pub region: Option<String>,
    pub need_default_resource: bool,
    pub nat_services: HashMap<NatServiceId, NatService>,
    pub volumes: HashMap<VolumeId, Volume>,
    pub snapshots: HashMap<SnapshotId, Snapshot>,
    pub fetch_date: Option<DateTime<Utc>>,
    pub public_ips: HashMap<PublicIpId, PublicIp>,
    pub filters: Option<Filter>,
    pub flexible_gpus: HashMap<FlexibleGpuId, FlexibleGpu>,
    pub load_balancers: Vec<LoadbalancerId>,
    pub vpns: Vec<VpnId>,
    pub buckets: HashMap<BucketId, OosBucket>,
    pub consumption: HashMap<CatalogId, ConsumptionEntry>,
}

impl Input {
    pub fn new(profile_name: Option<String>) -> Result<Input, Box<dyn error::Error>> {
        let (config, aws_config) = Input::get_config(profile_name)?;
        Ok(Input {
            config,
            aws_config,
            vms: HashMap::new(),
            vms_images: HashMap::new(),
            catalog: HashMap::new(),
            need_vm_types_fetch: false,
            vm_types: HashMap::new(),
            account: None,
            region: None,
            need_default_resource: false,
            volumes: HashMap::<VolumeId, Volume>::new(),
            snapshots: HashMap::<SnapshotId, Snapshot>::new(),
            nat_services: HashMap::<NatServiceId, NatService>::new(),
            fetch_date: None,
            public_ips: HashMap::new(),
            filters: None,
            flexible_gpus: HashMap::new(),
            use_dedicated_instance: false,
            load_balancers: Vec::new(),
            vpns: Vec::new(),
            buckets: HashMap::new(),
            consumption: HashMap::new(),
        })
    }

    fn get_config(profile: Option<String>) -> Result<(Configuration, SdkConfig), Box<dyn Error>> {
        // Test if the 'profile' parameter is set
        trace!("try to load api config parameter");
        if let Some(profile_name) = profile {
            let config_file = ConfigurationFile::load_default()?;
            let mut config = config_file.configuration(&profile_name)?;
            config.user_agent = Some(format!("osc-cost/{VERSION}"));

            return Ok((
                config,
                Input::build_aws_config_from_file(&config_file, profile_name)?,
            ));
        }

        // If not, check for environment variables
        trace!("try to load api config from environment variables");
        let ak_env = env::var("OSC_ACCESS_KEY").ok();
        let sk_env = env::var("OSC_SECRET_KEY").ok();
        let region_env = env::var("OSC_REGION").ok();
        match (ak_env, sk_env, region_env) {
            (Some(access_key), Some(secret_key), Some(region)) => {
                let mut config = Configuration::new();
                config.base_path = format!("https://api.{region}.outscale.com/api/v1");
                config.aws_v4_key = Some(AWSv4Key {
                    region: region.clone(),
                    access_key: access_key.clone(),
                    secret_key: secret_key.clone().into(),
                    service: "oapi".to_string(),
                });
                config.user_agent = Some(format!("osc-cost/{VERSION}"));
                return Ok((
                    config,
                    Input::build_aws_config(access_key, secret_key, region),
                ));
            }
            (None, None, None) => {}
            (_, _, _) => {
                warn!("some credentials are set through environement variable but not all. OSC_ACCESS_KEY, OSC_SECRET_KEY and OSC_REGION are required to use this method.");
            }
        };

        // If not, check 'default' profile
        trace!("try to load default config from configuration file");
        let profile_name = "default";
        trace!("try to load api config from configuration file");
        let config_file = ConfigurationFile::load_default()?;
        let mut config = config_file.configuration(profile_name)?;
        config.user_agent = Some(format!("osc-cost/{VERSION}"));

        Ok((
            config,
            Input::build_aws_config_from_file(&config_file, profile_name)?,
        ))
    }

    fn build_aws_config(ak: String, sk: String, region: String) -> SdkConfig {
        let cred = Credentials::new(ak, sk, None, None, "oapi");
        // TODO: set Appname
        aws_config::SdkConfig::builder()
            .endpoint_url(format!("https://oos.{region}.outscale.com"))
            .behavior_version(BehaviorVersion::v2025_08_07())
            .region(Region::new(region))
            .credentials_provider(SharedCredentialsProvider::new(cred))
            .build()
    }

    fn build_aws_config_from_file<S: Into<String>>(
        config_file: &ConfigurationFile,
        profile_name: S,
    ) -> Result<SdkConfig, Box<dyn Error>> {
        let profile_name = profile_name.into();
        let profile = match config_file.0.get(&profile_name) {
            Some(profile) => profile.clone(),
            None => return Err(Box::new(ConfigurationFileError::ProfileNotFound)),
        };

        let region = profile.region.ok_or("No region for the profile")?;
        let access_key = profile.access_key.ok_or("No AK for the profile")?;
        let secret_key = profile.secret_key.ok_or("No SK for the profile")?;

        Ok(Input::build_aws_config(access_key, secret_key, region))
    }

    pub fn fetch(&mut self) -> Result<(), Box<dyn error::Error>> {
        self.fetch_date = Some(Utc::now());
        self.fetch_vms()?;
        self.fetch_vms_images()?;
        self.fetch_catalog()?;
        if self.need_vm_types_fetch {
            self.fetch_vm_types()?;
        }
        self.fetch_dedicated_instances()?;
        self.fetch_account()?;
        self.fetch_region()?;
        self.fetch_volumes()?;
        self.fetch_nat_services()?;
        self.fetch_public_ips()?;
        self.fetch_snapshots()?;
        self.fetch_flexible_gpus()?;
        self.fetch_load_balancers()?;
        self.fetch_vpns()?;
        self.fetch_buckets()?;
        Ok(())
    }

    pub fn fetch_catalog(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadCatalogResponse = {
            let request = ReadCatalogRequest::new();
            read_catalog(&self.config, Some(request))?
        };
        debug!("{:#?}", result);

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
            let entry_id = format!("{service}/{_type}/{operation}");
            self.catalog.insert(entry_id, entry);
        }

        info!("fetched {} catalog entries", self.catalog.len());
        Ok(())
    }

    fn fetch_account(&mut self) -> Result<(), Box<dyn error::Error>> {
        let result: ReadAccountsResponse = {
            let request = ReadAccountsRequest::new();
            read_accounts(&self.config, Some(request))?
        };
        debug!("{:#?}", result);

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
        let result: ReadSubregionsResponse = {
            let request = ReadSubregionsRequest::new();
            read_subregions(&self.config, Some(request))?
        };
        debug!("{:#?}", result);

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

    fn skip_fetch<S: Into<String>>(&self, resource_type: S) -> bool {
        if let Some(filters) = &self.filters {
            return filters.skip_resource.contains(&resource_type.into());
        }
        false
    }

    pub fn build_resources(&mut self) -> Resources {
        let mut resources = Resources {
            resources: Vec::new(),
        };
        self.fill_resource_vm(&mut resources);
        self.fill_resource_volume(&mut resources);
        self.fill_resource_public_ip(&mut resources);
        self.fill_resource_snapshot(&mut resources);
        self.fill_resource_nat_service(&mut resources);
        self.fill_resource_flexible_gpus(&mut resources);
        self.fill_resource_load_balancers(&mut resources);
        self.fill_resource_vpns(&mut resources);
        self.fill_resource_oos(&mut resources);
        self.fill_resource_dedicated_instances(&mut resources);
        resources
    }
}

impl From<Input> for Resources {
    fn from(mut input: Input) -> Self {
        input.build_resources()
    }
}
