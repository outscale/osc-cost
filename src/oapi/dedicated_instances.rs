use std::error;

use log::{info, warn};

use crate::{
    core::{dedicated_instances::DedicatedInstance, Resource, Resources},
    VERSION,
};

use super::Input;

impl Input {
    pub fn fetch_dedicated_instances(&self) -> Result<(), Box<dyn error::Error>> {
        if self.use_dedicated_instance {
            info!("Use dedicated instance")
        }
        Ok(())
    }
    pub fn fill_resource_dedicated_instances(&self, resources: &mut Resources) {
        if self.use_dedicated_instance {
            let Some(price_per_hour) =
                self.catalog_entry("TinaOS-FCU", "UseDedicated", "RunDedicatedInstances")
            else {
                warn!("warning: could not retrieve catalog for dedicated instance");
                return;
            };
            let core_resource = DedicatedInstance {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                price_per_hour: Some(price_per_hour),
                price_per_month: None,
            };
            resources
                .resources
                .push(Resource::DedicatedInstance(core_resource));
        } else {
            info!("Use default instance")
        }
    }
}
