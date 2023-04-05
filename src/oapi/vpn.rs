use std::error;

use log::warn;
use outscale_api::{
    apis::vpn_connection_api::read_vpn_connections,
    models::{FiltersVpnConnection, ReadVpnConnectionsRequest, ReadVpnConnectionsResponse},
};

use crate::{
    core::{vpn::Vpn, Resource, Resources},
    VERSION,
};

use super::Input;

pub type VpnId = String;
const RESOURCE_NAME: &str = "Vpn";

impl Input {
    pub fn fetch_vpns(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let filters = match &self.filters {
            Some(filter) => FiltersVpnConnection {
                tag_keys: Some(filter.tag_keys.clone()),
                tag_values: Some(filter.tag_values.clone()),
                tags: Some(filter.tags.clone()),
                ..Default::default()
            },
            None => FiltersVpnConnection::new(),
        };
        let request = ReadVpnConnectionsRequest {
            filters: Some(Box::new(filters)),
            ..Default::default()
        };
        let result: ReadVpnConnectionsResponse = loop {
            let response = read_vpn_connections(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let resources = match result.vpn_connections {
            None => {
                warn!("warning: no vpn available");
                return Ok(());
            }
            Some(vpn) => vpn,
        };
        for vpn in resources {
            let vpn_id = vpn
                .vpn_connection_id
                .clone()
                .unwrap_or_else(|| String::from(""));
            self.vpns.push(vpn_id);
        }
        warn!("info: fetched {} vpns", self.vpns.len());
        Ok(())
    }

    pub fn fill_resource_vpns(&self, resources: &mut Resources) {
        let Some(price_per_hour) = self.catalog_entry(
            "TinaOS-FCU",
            "ConnectionUsage",
            "CreateVpnConnection",
        ) else {
            warn!("warning: could not retrieve the catalog for vpn");
            return;
        };
        for resource_id in &self.vpns {
            let core_resource = Vpn {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(resource_id.clone()),
                price_per_hour: Some(price_per_hour),
                price_per_month: None,
            };
            resources.resources.push(Resource::Vpn(core_resource));
        }
    }
}
