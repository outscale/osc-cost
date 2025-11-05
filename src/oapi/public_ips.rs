use std::error;

use log::{debug, info, warn};
use outscale_api::{
    apis::public_ip_api::read_public_ips,
    models::{FiltersPublicIp, ReadPublicIpsRequest, ReadPublicIpsResponse},
};

use crate::{
    core::{public_ips::PublicIp, Resource, Resources},
    VERSION,
};

use super::Input;

pub type PublicIpId = String;
const RESOURCE_NAME: &str = "PublicIp";

impl Input {
    pub fn fetch_public_ips(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let filters: FiltersPublicIp = match &self.filters {
            Some(filter) => FiltersPublicIp {
                tag_keys: Some(filter.tag_keys.clone()),
                tag_values: Some(filter.tag_values.clone()),
                tags: Some(filter.tags.clone()),
                ..Default::default()
            },
            None => FiltersPublicIp::new(),
        };
        let request = ReadPublicIpsRequest {
            filters: Some(Box::new(filters)),
            ..Default::default()
        };
        let result: ReadPublicIpsResponse = read_public_ips(&self.config, Some(request.clone()))?;
        debug!("{:#?}", result);

        let public_ips = match result.public_ips {
            None => {
                warn!("no public ip list provided");
                return Ok(());
            }
            Some(ips) => ips,
        };
        self.public_ips.clear();
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

    pub fn fill_resource_public_ip(&self, resources: &mut Resources) {
        if self.public_ips.is_empty() && self.need_default_resource {
            resources.resources.push(Resource::PublicIp(PublicIp {
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                ..Default::default()
            }));
        }
        for (public_ip_id, public_ip) in &self.public_ips {
            let mut price_non_attached: Option<f32> = None;
            let mut price_first_ip: Option<f32> = None;
            let mut price_next_ips: Option<f32> = None;
            let Some(public_ip_str) = &public_ip.public_ip else {
                warn!("cannot get public ip content for {}", public_ip_id);
                continue;
            };

            match (&public_ip.link_public_ip_id, &public_ip.vm_id) {
                (None, None) => match self.catalog_entry(
                    "TinaOS-FCU",
                    "ElasticIP:IdleAddress",
                    "AssociateAddressVPC",
                ) {
                    Some(price) => price_non_attached = Some(price),
                    None => continue,
                },
                (Some(_), None) => continue,
                (Some(_), Some(vm_id)) => match self.vms.get(vm_id) {
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
                (None, Some(_)) => {
                    warn!("cannot have a VmId and no link");
                    continue;
                }
            };
            let core_public_ip = PublicIp {
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
            resources.resources.push(Resource::PublicIp(core_public_ip));
        }
    }
}
