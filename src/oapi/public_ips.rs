use std::error;

use log::{info, warn};
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

impl Input {
    pub fn fetch_public_ips(&mut self) -> Result<(), Box<dyn error::Error>> {
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

    pub fn fill_resource_public_ip(&self, resources: &mut Resources) {
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
