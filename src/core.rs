use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::error;
use std::fmt;
use strum_macros::EnumString;

use self::dedicated_instances::DedicatedInstance;
use self::flexible_gpus::FlexibleGpu;
use self::load_balancers::LoadBalancer;
use self::nat_services::NatServices;
use self::oos::Oos;
use self::public_ips::PublicIp;
use self::snapshots::Snapshot;
use self::vms::Vm;
use self::volumes::Volume;
use self::vpn::Vpn;

static HOURS_PER_MONTH: f32 = (365_f32 * 24_f32) / 12_f32;

pub mod dedicated_instances;
pub mod digest;
pub mod flexible_gpus;
pub mod load_balancers;
pub mod nat_services;
pub mod oos;
pub mod public_ips;
pub mod snapshots;
pub mod vms;
pub mod volumes;
pub mod vpn;

#[derive(Serialize, Deserialize, Debug, EnumString)]
#[serde(tag = "resource_type")]
pub enum Resource {
    Vm(Vm),
    Volume(Volume),
    PublicIp(PublicIp),
    Snapshot(Snapshot),
    NatServices(NatServices),
    Aggregate(Aggregate),
    FlexibleGpu(FlexibleGpu),
    LoadBalancer(LoadBalancer),
    Vpn(Vpn),
    Oos(Oos),
    DedicatedInstance(DedicatedInstance),
}

#[derive(Debug, Serialize)]
pub struct Resources {
    pub resources: Vec<Resource>,
}

impl Resources {
    pub fn compute(&mut self) -> Result<(), ResourceError> {
        for resource in self.resources.iter_mut() {
            match resource {
                Resource::Volume(volume) => volume.compute()?,
                Resource::Vm(vm) => vm.compute()?,
                Resource::PublicIp(pip) => pip.compute()?,
                Resource::Snapshot(snapshot) => snapshot.compute()?,
                Resource::NatServices(nat_service) => nat_service.compute()?,
                Resource::Aggregate(aggregate) => aggregate.compute()?,
                Resource::FlexibleGpu(flexible_gpu) => flexible_gpu.compute()?,
                Resource::LoadBalancer(load_balancer) => load_balancer.compute()?,
                Resource::Vpn(vpn) => vpn.compute()?,
                Resource::Oos(oos) => oos.compute()?,
                Resource::DedicatedInstance(dedicated_instance) => dedicated_instance.compute()?,
            }
        }
        Ok(())
    }

    pub fn aggregate(self) -> Self {
        let mut resource_aggregate: HashMap<String, Aggregate> = HashMap::new();

        for resource in self.resources {
            let aggregate: Aggregate = Aggregate::from(resource);
            if let Some(cache) = resource_aggregate.get_mut(&aggregate.aggregated_resource_type) {
                cache.price_per_hour = match cache.price_per_hour {
                    Some(price) => Some(price + aggregate.price_per_hour.unwrap_or(0.0)),
                    None => aggregate.price_per_hour,
                };

                cache.price_per_month = match cache.price_per_month {
                    Some(price) => Some(price + aggregate.price_per_month.unwrap_or(0.0)),
                    None => aggregate.price_per_month,
                };

                cache.count += aggregate.count;
            } else {
                resource_aggregate.insert(aggregate.aggregated_resource_type.clone(), aggregate);
            }
        }

        let mut result = Resources {
            resources: Vec::new(),
        };

        for val in resource_aggregate.values() {
            result.resources.push(Resource::Aggregate(val.clone()));
        }

        result
    }

    pub fn cost_per_hour(&self) -> Result<f32, ResourceError> {
        let mut total = 0f32;
        for resource in &self.resources {
            match resource {
                Resource::Volume(volume) => {
                    total += volume.price_per_hour()?;
                }
                Resource::Vm(vm) => {
                    total += vm.price_per_hour()?;
                }
                Resource::PublicIp(pip) => {
                    total += pip.price_per_hour()?;
                }
                Resource::Snapshot(snapshot) => {
                    total += snapshot.price_per_hour()?;
                }
                Resource::NatServices(nat_services) => {
                    total += nat_services.price_per_hour()?;
                }
                Resource::Aggregate(aggregade) => {
                    total += aggregade.price_per_hour()?;
                }
                Resource::FlexibleGpu(flexible_gpu) => {
                    total += flexible_gpu.price_per_hour()?;
                }
                Resource::LoadBalancer(load_balancer) => {
                    total += load_balancer.price_per_hour()?;
                }
                Resource::Vpn(vpn) => {
                    total += vpn.price_per_hour()?;
                }
                Resource::Oos(oos) => {
                    total += oos.price_per_hour()?;
                }
                Resource::DedicatedInstance(dedicated_instance) => {
                    total += dedicated_instance.price_per_hour()?;
                }
            }
        }
        Ok(total)
    }

    pub fn cost_per_month(&self) -> Result<f32, ResourceError> {
        Ok(self.cost_per_hour()? * HOURS_PER_MONTH)
    }

    pub fn cost_per_year(&self) -> Result<f32, ResourceError> {
        Ok(self.cost_per_hour()? * HOURS_PER_MONTH * 12.0)
    }
}

#[derive(Debug, Clone)]
pub enum ResourceError {
    NotComputed,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResourceError::NotComputed => write!(f, "resource price is not computed yet"),
        }
    }
}

impl error::Error for ResourceError {}

trait ResourceTrait {
    fn price_per_hour(&self) -> Result<f32, ResourceError>;
    fn compute(&mut self) -> Result<(), ResourceError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Aggregate {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub aggregated_resource_type: String,
    pub count: i32,
}

impl ResourceTrait for Aggregate {
    fn price_per_hour(&self) -> Result<f32, ResourceError> {
        match self.price_per_hour {
            Some(price) => Ok(price),
            None => Err(ResourceError::NotComputed),
        }
    }

    fn compute(&mut self) -> Result<(), ResourceError> {
        Ok(())
    }
}

impl From<Resource> for Aggregate {
    fn from(item: Resource) -> Self {
        match item {
            Resource::Vm(vm) => Aggregate {
                osc_cost_version: vm.osc_cost_version,
                account_id: vm.account_id,
                read_date_rfc3339: vm.read_date_rfc3339,
                region: vm.region,
                price_per_hour: vm.price_per_hour,
                price_per_month: vm.price_per_month,
                aggregated_resource_type: "Vm".to_string(),
                count: 1,
            },
            Resource::Volume(volume) => Aggregate {
                osc_cost_version: volume.osc_cost_version,
                account_id: volume.account_id,
                read_date_rfc3339: volume.read_date_rfc3339,
                region: volume.region,
                price_per_hour: volume.price_per_hour,
                price_per_month: volume.price_per_month,
                aggregated_resource_type: "Volume".to_string(),
                count: 1,
            },
            Resource::PublicIp(public_ip) => Aggregate {
                osc_cost_version: public_ip.osc_cost_version,
                account_id: public_ip.account_id,
                read_date_rfc3339: public_ip.read_date_rfc3339,
                region: public_ip.region,
                price_per_hour: public_ip.price_per_hour,
                price_per_month: public_ip.price_per_month,
                aggregated_resource_type: "PublicIp".to_string(),
                count: 1,
            },
            Resource::Snapshot(snapshot) => Aggregate {
                osc_cost_version: snapshot.osc_cost_version,
                account_id: snapshot.account_id,
                read_date_rfc3339: snapshot.read_date_rfc3339,
                region: snapshot.region,
                price_per_hour: snapshot.price_per_hour,
                price_per_month: snapshot.price_per_month,
                aggregated_resource_type: "Snapshot".to_string(),
                count: 1,
            },
            Resource::NatServices(nat_service) => Aggregate {
                osc_cost_version: nat_service.osc_cost_version,
                account_id: nat_service.account_id,
                read_date_rfc3339: nat_service.read_date_rfc3339,
                region: nat_service.region,
                price_per_hour: nat_service.price_per_hour,
                price_per_month: nat_service.price_per_month,
                aggregated_resource_type: "NatServices".to_string(),
                count: 1,
            },
            Resource::DedicatedInstance(dedicated_instance) => Aggregate {
                osc_cost_version: dedicated_instance.osc_cost_version,
                account_id: dedicated_instance.account_id,
                read_date_rfc3339: dedicated_instance.read_date_rfc3339,
                region: dedicated_instance.region,
                price_per_hour: dedicated_instance.price_per_hour,
                price_per_month: dedicated_instance.price_per_month,
                aggregated_resource_type: "DedicatedInstance".to_string(),
                count: 1,
            },
            Resource::Aggregate(aggregate) => aggregate,
            Resource::FlexibleGpu(flexible_gpu) => Aggregate {
                osc_cost_version: flexible_gpu.osc_cost_version,
                account_id: flexible_gpu.account_id,
                read_date_rfc3339: flexible_gpu.read_date_rfc3339,
                region: flexible_gpu.region,
                price_per_hour: flexible_gpu.price_per_hour,
                price_per_month: flexible_gpu.price_per_month,
                aggregated_resource_type: "FlexibleGpu".to_string(),
                count: 1,
            },
            Resource::LoadBalancer(load_balancer) => Aggregate {
                osc_cost_version: load_balancer.osc_cost_version,
                account_id: load_balancer.account_id,
                read_date_rfc3339: load_balancer.read_date_rfc3339,
                region: load_balancer.region,
                price_per_hour: load_balancer.price_per_hour,
                price_per_month: load_balancer.price_per_month,
                aggregated_resource_type: "LoadBalancer".to_string(),
                count: 1,
            },
            Resource::Vpn(resource) => Aggregate {
                osc_cost_version: resource.osc_cost_version,
                account_id: resource.account_id,
                read_date_rfc3339: resource.read_date_rfc3339,
                region: resource.region,
                price_per_hour: resource.price_per_hour,
                price_per_month: resource.price_per_month,
                aggregated_resource_type: "Vpn".to_string(),
                count: 1,
            },
            Resource::Oos(resource) => Aggregate {
                osc_cost_version: resource.osc_cost_version,
                account_id: resource.account_id,
                read_date_rfc3339: resource.read_date_rfc3339,
                region: resource.region,
                price_per_hour: resource.price_per_hour,
                price_per_month: resource.price_per_month,
                aggregated_resource_type: "Oos".to_string(),
                count: 1,
            },
        }
    }
}
