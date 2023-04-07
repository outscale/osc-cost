use std::collections::HashMap;

use chrono::NaiveDate;
use log::warn;
use serde::{Deserialize, Serialize};

use super::{Resource, Resources};
use std::error::Error;

#[derive(Clone, Debug, Default)]
pub struct Digest {
    pub price: Option<f32>,
}

pub fn match_entry_id_resource_type(entry_id: &String) -> Option<String> {
    match entry_id {
        // VM
        s if s.starts_with("TinaOS-FCU/BoxUsage") => Some(String::from("Vm")),
        s if s.starts_with("TinaOS-FCU/CustomCore") => Some(String::from("Vm")),
        s if s.starts_with("TinaOS-FCU/CustomRam") => Some(String::from("Vm")),
        s if s.starts_with("TinaOS-FCU/ProductUsage") => Some(String::from("Vm")),
        // Volume
        s if s.starts_with("TinaOS-FCU/BSU") => Some(String::from("Volume")),
        // VPN
        s if s == "TinaOS-FCU/ConnectionUsage/CreateVpnConnection" => Some(String::from("Vpn")),
        // Public IP
        s if s.starts_with("TinaOS-FCU/ElasticIP") => Some(String::from("PublicIp")),
        // NAT Services
        s if s == "TinaOS-FCU/NatGatewayUsage/CreateNatGateway" => {
            Some(String::from("NatServices"))
        }
        // Snapshots
        s if s == "TinaOS-FCU/Snapshot:Usage/Snapshot" => Some(String::from("Snapshot")),
        // OOS
        s if s.starts_with("TinaOS-OOS") => Some(String::from("Oos")),
        s if s.starts_with("TinaOS-OSU") => Some(String::from("Oos")),
        // LBU
        s if s == "TinaOS-LBU/LBU:Usage/CreateLoadBalancer" => Some(String::from("LoadBalancer")),
        _ => {
            warn!("Entryid {} does not match any resources", entry_id);
            None
        }
    }
}

pub struct Drifts {
    pub drifts: Vec<Drift>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Drift {
    pub category: String,
    pub osc_cost_price: f32,
    pub digest_price: f32,
    pub drift: i32,
}

pub fn compute_drift(
    digest: HashMap<String, Digest>,
    resources: &Resources,
    from_date: &str,
    to_date: &str,
) -> Result<Drifts, Box<dyn Error>> {
    let mut drifts = Vec::<Drift>::new();

    let from_date = NaiveDate::parse_from_str(from_date, "%Y-%m-%d")?;
    let to_date = NaiveDate::parse_from_str(to_date, "%Y-%m-%d")?;
    let diff = (to_date - from_date).num_hours() as f32;

    for resource in resources.resources.iter() {
        match resource {
            Resource::Aggregate(osc_cost) => {
                let (digest_price, drift) = match digest.get(&osc_cost.aggregated_resource_type) {
                    Some(digest) => {
                        let Some(price) = digest.price else {
                            warn!("the digest price for this resource {} has not been computed", &osc_cost.aggregated_resource_type);
                            continue;
                        };

                        let Some(mut osc_cost_price) = osc_cost.price_per_hour else {
                            warn!("the osc_cost price for this resource {} has not been computed", &osc_cost.aggregated_resource_type);
                            continue;
                        };

                        // Mulitply by the number of hours
                        osc_cost_price *= diff;

                        (price, ((osc_cost_price - price) * 100.0 / price) as i32)
                    }
                    // The digest did not compute anything for this resource
                    None => {
                        if osc_cost.price_per_hour.is_none() || osc_cost.price_per_hour == Some(0.0)
                        {
                            continue;
                        } else {
                            (0.0, 100)
                        }
                    }
                };

                drifts.push(Drift {
                    category: String::from(&osc_cost.aggregated_resource_type),
                    osc_cost_price: osc_cost.price_per_hour.unwrap_or(0.0) * diff,
                    digest_price,
                    drift,
                })
            }
            _ => {
                warn!("cannot handle non aggreagated resources");
                continue;
            }
        }
    }

    Ok(Drifts { drifts })
}
