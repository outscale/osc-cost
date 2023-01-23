use self::flexible_gpus::FlexibleGpu;
use self::flexible_gpus::FlexibleGpuMetrics;
use self::load_balancers::LoadBalancer;
use self::load_balancers::LoadBalancerMetrics;
use self::nat_services::NatServiceMetrics;
use self::nat_services::NatServices;
use self::oos::Oos;
use self::oos::OosMetrics;
use self::public_ips::PublicIp;
use self::public_ips::PublicIpMetrics;
use self::snapshots::Snapshot;
use self::snapshots::SnapshotMetrics;
use self::vms::Vm;
use self::vms::VmMetrics;
use self::volumes::Volume;
use self::volumes::VolumeMetrics;
use self::vpn::Vpn;
use self::vpn::VpnMetrics;
use log::warn;
use prometheus::core::AtomicF64;
use prometheus::core::GenericGauge;
use prometheus::Encoder;
use prometheus::Gauge;
use prometheus::Opts;
use prometheus::Registry;
use prometheus::TextEncoder;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::process;

static HOURS_PER_MONTH: f32 = (365_f32 * 24_f32) / 12_f32;

pub mod flexible_gpus;
pub mod load_balancers;
pub mod nat_services;
pub mod oos;
pub mod public_ips;
pub mod snapshots;
pub mod vms;
pub mod volumes;
pub mod vpn;

#[derive(Serialize, Deserialize, Debug)]
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
}

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
            }
        }
        Ok(total)
    }

    pub fn cost_per_month(&self) -> Result<f32, ResourceError> {
        Ok(self.cost_per_hour()? * HOURS_PER_MONTH)
    }

    pub fn json(&self) -> serde_json::Result<String> {
        let mut out = String::new();
        for resource in &self.resources {
            match serde_json::to_string(resource) {
                Ok(serialized) => out.push_str(serialized.as_str()),
                Err(e) => {
                    warn!("provide vm serialization: {}", e);
                    continue;
                }
            }
            out.push('\n');
        }
        out.pop();
        Ok(out)
    }

    pub fn csv(&self) -> Result<String, Box<dyn error::Error>> {
        let mut csv_writer = csv::WriterBuilder::new().flexible(true).from_writer(vec![]);
        for resource in &self.resources {
            csv_writer.serialize(resource)?;
        }
        let output = String::from_utf8(csv_writer.into_inner()?)?;
        Ok(output)
    }
    pub fn metrics(&self) -> Result<Registry, prometheus::Error> {
        let mut registry = Registry::new();

        for resource in &self.resources {
            match resource {
                Resource::Vm(vm) => {
                    let vm_price_per_hours = match vm.gauge_hour() {
                        Ok(vm_price_per_hours) => vm_price_per_hours,
                        Err(e) => {
                            warn!("provide vm price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let vm_price_per_months = match vm.gauge_month() {
                        Ok(vm_price_per_months) => vm_price_per_months,
                        Err(e) => {
                            warn!("provide vm price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let vm_metrics = VmMetrics {
                        vm_price_per_hours: vm_price_per_hours,
                        vm_price_per_months: vm_price_per_months,
                    };
                    let price_per_hour = vm.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = vm.price_per_month.unwrap_or_default() as f64;
                    registry = vm_metrics.register(registry)?;

                    vm_metrics.vm_price_per_hours.add(price_per_hour);
                    vm_metrics.vm_price_per_months.add(price_per_month);
                }
                Resource::Volume(volume) => {
                    let volume_price_per_hours = match volume.gauge_hour() {
                        Ok(volume_price_per_hours) => volume_price_per_hours,
                        Err(e) => {
                            warn!("provide volume price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let volume_price_per_months = match volume.gauge_month() {
                        Ok(volume_price_per_months) => volume_price_per_months,
                        Err(e) => {
                            warn!("provide volume price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let volume_metrics = VolumeMetrics {
                        volume_price_per_hours: volume_price_per_hours,
                        volume_price_per_months: volume_price_per_months,
                    };
                    let price_per_hour = volume.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = volume.price_per_month.unwrap_or_default() as f64;
                    registry = volume_metrics.register(registry)?;

                    volume_metrics.volume_price_per_hours.add(price_per_hour);
                    volume_metrics.volume_price_per_months.add(price_per_month);
                }

                Resource::PublicIp(public_ip) => {
                    let public_ip_price_per_hours = match public_ip.gauge_hour() {
                        Ok(public_ip_price_per_hours) => public_ip_price_per_hours,
                        Err(e) => {
                            warn!("provide public ip price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let public_ip_price_per_months = match public_ip.gauge_month() {
                        Ok(volume_price_per_months) => volume_price_per_months,
                        Err(e) => {
                            warn!("provide public ip price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let public_ip_metrics = PublicIpMetrics {
                        publicip_price_per_hours: public_ip_price_per_hours,
                        publicip_price_per_months: public_ip_price_per_months,
                    };
                    let price_per_hour = public_ip.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = public_ip.price_per_month.unwrap_or_default() as f64;
                    registry = public_ip_metrics.register(registry)?;
                    public_ip_metrics
                        .publicip_price_per_hours
                        .add(price_per_hour);
                    public_ip_metrics
                        .publicip_price_per_months
                        .add(price_per_month);
                }

                Resource::Snapshot(snapshot) => {
                    let snapshot_price_per_hours = match snapshot.gauge_hour() {
                        Ok(snapshot_price_per_hours) => snapshot_price_per_hours,
                        Err(e) => {
                            warn!("provide snapshot price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let snapshot_price_per_months = match snapshot.gauge_month() {
                        Ok(snapshot_price_per_months) => snapshot_price_per_months,
                        Err(e) => {
                            warn!("provide snapshot price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let snapshot_metrics = SnapshotMetrics {
                        snapshot_price_per_hours: snapshot_price_per_hours,
                        snapshot_price_per_months: snapshot_price_per_months,
                    };
                    let price_per_hour = snapshot.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = snapshot.price_per_month.unwrap_or_default() as f64;
                    registry = snapshot_metrics.register(registry)?;
                    snapshot_metrics
                        .snapshot_price_per_hours
                        .add(price_per_hour);
                    snapshot_metrics
                        .snapshot_price_per_months
                        .add(price_per_month);
                }
                Resource::NatServices(natservice) => {
                    let natservice_price_per_hours = match natservice.gauge_hour() {
                        Ok(natservice_price_per_hours) => natservice_price_per_hours,
                        Err(e) => {
                            warn!("provide nat service price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let natservice_price_per_months = match natservice.gauge_month() {
                        Ok(natservice_price_per_months) => natservice_price_per_months,
                        Err(e) => {
                            warn!("provide nat service price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let natservice_metrics = NatServiceMetrics {
                        nat_service_price_per_hours: natservice_price_per_hours,
                        nat_service_price_per_months: natservice_price_per_months,
                    };
                    let price_per_hour = natservice.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = natservice.price_per_month.unwrap_or_default() as f64;
                    registry = natservice_metrics.register(registry)?;
                    natservice_metrics
                        .nat_service_price_per_hours
                        .add(price_per_hour);
                    natservice_metrics
                        .nat_service_price_per_months
                        .add(price_per_month);
                }
                Resource::Aggregate(aggregate) => {
                    let aggregate_price_per_hours = match aggregate.gauge_hour() {
                        Ok(aggregate_price_per_hours) => aggregate_price_per_hours,
                        Err(e) => {
                            warn!("provide aggregate price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let aggregate_price_per_months = match aggregate.gauge_month() {
                        Ok(aggregate_price_per_months) => aggregate_price_per_months,
                        Err(e) => {
                            warn!("provide aggregate price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };

                    let aggregate_metrics = AggregateMetrics {
                        aggregate_price_per_hours: aggregate_price_per_hours,
                        aggregate_price_per_months: aggregate_price_per_months,
                    };
                    let price_per_hour = aggregate.price_per_hour.unwrap() as f64;
                    let price_per_month = aggregate.price_per_month.unwrap() as f64;
                    registry = aggregate_metrics.register(registry)?;

                    aggregate_metrics
                        .aggregate_price_per_hours
                        .add(price_per_hour);
                    aggregate_metrics
                        .aggregate_price_per_months
                        .add(price_per_month);
                }
                Resource::FlexibleGpu(flexible_gpu) => {
                    let flexible_gpu_price_per_hours = match flexible_gpu.gauge_hour() {
                        Ok(flexible_gpu_price_per_hours) => flexible_gpu_price_per_hours,
                        Err(e) => {
                            warn!("provide flexible gpu price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let flexible_gpu_price_per_months = match flexible_gpu.gauge_month() {
                        Ok(flexible_gpu_price_per_months) => flexible_gpu_price_per_months,
                        Err(e) => {
                            warn!("provide flexible gpu price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let flexible_gpu_metrics = FlexibleGpuMetrics {
                        flexible_gpu_price_per_hours: flexible_gpu_price_per_hours,
                        flexible_gpu_price_per_months: flexible_gpu_price_per_months,
                    };
                    let price_per_hour = flexible_gpu.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = flexible_gpu.price_per_month.unwrap_or_default() as f64;
                    registry = flexible_gpu_metrics.register(registry)?;
                    flexible_gpu_metrics
                        .flexible_gpu_price_per_hours
                        .add(price_per_hour);
                    flexible_gpu_metrics
                        .flexible_gpu_price_per_months
                        .add(price_per_month);
                }
                Resource::LoadBalancer(load_balancer) => {
                    let load_balancer_price_per_hours = match load_balancer.gauge_hour() {
                        Ok(load_balancer_price_per_hours) => load_balancer_price_per_hours,
                        Err(e) => {
                            warn!("provide load balancer price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let load_balancer_price_per_months = match load_balancer.gauge_month() {
                        Ok(load_balancer_price_per_months) => load_balancer_price_per_months,
                        Err(e) => {
                            warn!("provide load balancer price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let load_balancer_metrics = LoadBalancerMetrics {
                        load_balancer_price_per_hours: load_balancer_price_per_hours,
                        load_balancer_price_per_months: load_balancer_price_per_months,
                    };
                    let price_per_hour = load_balancer.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = load_balancer.price_per_month.unwrap_or_default() as f64;
                    registry = load_balancer_metrics.register(registry)?;
                    load_balancer_metrics
                        .load_balancer_price_per_hours
                        .add(price_per_hour);
                    load_balancer_metrics
                        .load_balancer_price_per_months
                        .add(price_per_month);
                }
                Resource::Vpn(vpn) => {
                    let vpn_price_per_hours = match vpn.gauge_hour() {
                        Ok(vpn_price_per_hours) => vpn_price_per_hours,
                        Err(e) => {
                            warn!("provide vpn price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let vpn_price_per_months = match vpn.gauge_month() {
                        Ok(vpn_price_per_months) => vpn_price_per_months,
                        Err(e) => {
                            warn!("provide vpn price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let vpn_metrics = VpnMetrics {
                        vpn_price_per_hours: vpn_price_per_hours,
                        vpn_price_per_months: vpn_price_per_months,
                    };
                    let price_per_hour = vpn.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = vpn.price_per_month.unwrap_or_default() as f64;
                    registry = vpn_metrics.register(registry)?;
                    vpn_metrics.vpn_price_per_hours.add(price_per_hour);
                    vpn_metrics.vpn_price_per_months.add(price_per_month);
                }
                Resource::Oos(oos) => {
                    let oos_price_per_hours = match oos.gauge_hour() {
                        Ok(vpn_price_per_hours) => vpn_price_per_hours,
                        Err(e) => {
                            warn!("provide oos price hour: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let oos_price_per_months = match oos.gauge_month() {
                        Ok(oos_price_per_months) => oos_price_per_months,
                        Err(e) => {
                            warn!("provide oos price month: {}", e);
                            GenericGauge::new("error", "error").unwrap()
                        }
                    };
                    let oos_metrics = OosMetrics {
                        oos_price_per_hours: oos_price_per_hours,
                        oos_price_per_months: oos_price_per_months,
                    };
                    let price_per_hour = oos.price_per_hour.unwrap_or_default() as f64;
                    let price_per_month = oos.price_per_month.unwrap_or_default() as f64;
                    registry = oos_metrics.register(registry)?;
                    oos_metrics.oos_price_per_hours.add(price_per_hour);
                    oos_metrics.oos_price_per_months.add(price_per_month);
                }
            }
        }
        Ok(registry)
    }
    pub fn get_body(registry: Registry) -> Option<String> {
        let mut buffer = Vec::<u8>::new();
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        Some(String::from_utf8(buffer.clone()).unwrap())
    }
    pub fn prometheus(&self) -> Result<String, prometheus::Error> {
        let registry = self.metrics()?;

        let body = match Resources::get_body(registry) {
            Some(body) => body,
            None => "".to_string(),
        };
        Ok(body)
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
    fn gauge_hour(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error>;
    fn gauge_month(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error>;
}

trait ResourceMetricsTrait {
    fn register(&self, registry: Registry) -> Result<Registry, prometheus::Error>;
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Aggregate {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub aggregated_resource_type: String,
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

    fn gauge_hour(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let aggregate_gauge_hour_opts =
            Opts::new("aggregate_price_hour", "Aggregate price by hour")
                .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
                .const_label("account_id", self.account_id.as_ref().unwrap())
                .const_label("region", self.region.as_ref().unwrap())
                .const_label("resource_id", "aggregate".to_string())
                .const_label("resource_type", "aggregate".to_string());
        let aggregate_gauge_hour =
            Gauge::with_opts(aggregate_gauge_hour_opts).unwrap_or_else(|err| {
                println!("Error aggregate gauge hour: {}", err);
                process::exit(1);
            });
        Ok(aggregate_gauge_hour)
    }

    fn gauge_month(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let aggregate_gauge_month_opts =
            Opts::new("aggregate_price_month", "Aggregate price by month")
                .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
                .const_label("account_id", self.account_id.as_ref().unwrap())
                .const_label("region", self.region.as_ref().unwrap())
                .const_label("resource_id", "aggregate".to_string())
                .const_label("resource_type", "aggregate".to_string());
        let aggregate_gauge_month =
            Gauge::with_opts(aggregate_gauge_month_opts).unwrap_or_else(|err| {
                println!("Error aggregate gauge month: {}", err);
                process::exit(1);
            });
        Ok(aggregate_gauge_month)
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
            },
            Resource::Volume(volume) => Aggregate {
                osc_cost_version: volume.osc_cost_version,
                account_id: volume.account_id,
                read_date_rfc3339: volume.read_date_rfc3339,
                region: volume.region,
                price_per_hour: volume.price_per_hour,
                price_per_month: volume.price_per_month,
                aggregated_resource_type: "Volume".to_string(),
            },
            Resource::PublicIp(public_ip) => Aggregate {
                osc_cost_version: public_ip.osc_cost_version,
                account_id: public_ip.account_id,
                read_date_rfc3339: public_ip.read_date_rfc3339,
                region: public_ip.region,
                price_per_hour: public_ip.price_per_hour,
                price_per_month: public_ip.price_per_month,
                aggregated_resource_type: "PublicIp".to_string(),
            },
            Resource::Snapshot(snapshot) => Aggregate {
                osc_cost_version: snapshot.osc_cost_version,
                account_id: snapshot.account_id,
                read_date_rfc3339: snapshot.read_date_rfc3339,
                region: snapshot.region,
                price_per_hour: snapshot.price_per_hour,
                price_per_month: snapshot.price_per_month,
                aggregated_resource_type: "Snapshot".to_string(),
            },
            Resource::NatServices(nat_service) => Aggregate {
                osc_cost_version: nat_service.osc_cost_version,
                account_id: nat_service.account_id,
                read_date_rfc3339: nat_service.read_date_rfc3339,
                region: nat_service.region,
                price_per_hour: nat_service.price_per_hour,
                price_per_month: nat_service.price_per_month,
                aggregated_resource_type: "NatServices".to_string(),
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
            },
            Resource::LoadBalancer(load_balancer) => Aggregate {
                osc_cost_version: load_balancer.osc_cost_version,
                account_id: load_balancer.account_id,
                read_date_rfc3339: load_balancer.read_date_rfc3339,
                region: load_balancer.region,
                price_per_hour: load_balancer.price_per_hour,
                price_per_month: load_balancer.price_per_month,
                aggregated_resource_type: "LoadBalancer".to_string(),
            },
            Resource::Vpn(resource) => Aggregate {
                osc_cost_version: resource.osc_cost_version,
                account_id: resource.account_id,
                read_date_rfc3339: resource.read_date_rfc3339,
                region: resource.region,
                price_per_hour: resource.price_per_hour,
                price_per_month: resource.price_per_month,
                aggregated_resource_type: "Vpn".to_string(),
            },
            Resource::Oos(resource) => Aggregate {
                osc_cost_version: resource.osc_cost_version,
                account_id: resource.account_id,
                read_date_rfc3339: resource.read_date_rfc3339,
                region: resource.region,
                price_per_hour: resource.price_per_hour,
                price_per_month: resource.price_per_month,
                aggregated_resource_type: "Oos".to_string(),
            },
        }
    }
}

pub struct AggregateMetrics {
    aggregate_price_per_hours: GenericGauge<AtomicF64>,
    aggregate_price_per_months: GenericGauge<AtomicF64>,
}
impl ResourceMetricsTrait for AggregateMetrics {
    fn register(&self, registry: Registry) -> Result<Registry, prometheus::Error> {
        registry
            .register(Box::new(self.aggregate_price_per_hours.clone()))
            .or_else(|e| Err(e))?;
        registry
            .register(Box::new(self.aggregate_price_per_months.clone()))
            .or_else(|e| Err(e))?;
        Ok(registry)
    }
}
