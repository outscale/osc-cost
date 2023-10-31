use std::error;

use log::warn;
use outscale_api::{
    apis::load_balancer_api::read_load_balancers,
    models::{ReadLoadBalancersRequest, ReadLoadBalancersResponse},
};

use crate::{
    core::{load_balancers::LoadBalancer, Resource, Resources},
    VERSION,
};

use super::Input;

pub type LoadbalancerId = String;
const RESOURCE_NAME: &str = "LoadBalancer";

impl Input {
    pub fn fetch_load_balancers(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let request = ReadLoadBalancersRequest {
            ..Default::default()
        };
        let result: ReadLoadBalancersResponse = loop {
            let response = read_load_balancers(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let resources = match result.load_balancers {
            None => {
                warn!("warning: no load balancer available");
                return Ok(());
            }
            Some(lbu) => lbu,
        };
        for lbu in resources {
            let lbu_id = lbu
                .load_balancer_name
                .clone()
                .unwrap_or_else(|| String::from(""));
            self.load_balancers.push(lbu_id);
        }
        warn!("info: fetched {} load balancers", self.load_balancers.len());
        Ok(())
    }

    pub fn fill_resource_load_balancers(&self, resources: &mut Resources) {
        if self.load_balancers.is_empty() && self.need_default_resource {
            resources
                .resources
                .push(Resource::LoadBalancer(LoadBalancer {
                    account_id: self.account_id(),
                    read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                    region: self.region.clone(),
                    ..Default::default()
                }));
        }
        let Some(price_per_hour) =
            self.catalog_entry("TinaOS-LBU", "LBU:Usage", "CreateLoadBalancer")
        else {
            warn!("warning: could not retrieve the catalog for load balancer");
            return;
        };
        for resource_id in &self.load_balancers {
            let core_resource = LoadBalancer {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(resource_id.clone()),
                price_per_hour: Some(price_per_hour),
                price_per_month: None,
            };
            resources
                .resources
                .push(Resource::LoadBalancer(core_resource));
        }
    }
}
