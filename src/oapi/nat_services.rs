use std::error;

use log::{info, warn};
use outscale_api::{
    apis::nat_service_api::read_nat_services,
    models::{FiltersNatService, ReadNatServicesRequest, ReadNatServicesResponse},
};

use crate::{
    core::{nat_services::NatServices, Resource, Resources},
    VERSION,
};

use super::Input;

pub type NatServiceId = String;

impl Input {
    pub fn fetch_nat_services(&mut self) -> Result<(), Box<dyn error::Error>> {
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

    pub fn fill_resource_nat_service(&self, resources: &mut Resources) {
        for (nat_service_id, nat_service) in &self.nat_services {
            let price_product_per_nat_service_per_hour =
                self.catalog_entry("TinaOS-FCU", "NatGatewayUsage", "CreateNatGateway");
            let Some(nat_service_id) = &nat_service.nat_service_id else {
                warn!("cannot get nat_service_id content for {}", nat_service_id );
                continue;
            };
            let core_nat_service = NatServices {
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
                .push(Resource::NatServices(core_nat_service));
        }
    }
}
