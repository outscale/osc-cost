use std::error;

use log::warn;
use outscale_api::{
    apis::flexible_gpu_api::read_flexible_gpus,
    models::{ReadFlexibleGpusRequest, ReadFlexibleGpusResponse},
};

use crate::{
    core::{flexible_gpus::FlexibleGpu, Resource, Resources},
    VERSION,
};

use super::Input;

pub type FlexibleGpuId = String;

const RESOURCE_NAME: &str = "FlexibleGpu";

impl Input {
    pub fn fetch_flexible_gpus(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let request = ReadFlexibleGpusRequest {
            ..Default::default()
        };
        let result: ReadFlexibleGpusResponse = loop {
            let response = read_flexible_gpus(&self.config, Some(request.clone()));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let flexible_gpus = match result.flexible_gpus {
            None => {
                warn!("warning: no flexible gpu available");
                return Ok(());
            }
            Some(flexible_gpus) => flexible_gpus,
        };
        for flexible_gpu in flexible_gpus {
            let flexible_gpu_id = flexible_gpu
                .flexible_gpu_id
                .clone()
                .unwrap_or_else(|| String::from(""));
            self.flexible_gpus.insert(flexible_gpu_id, flexible_gpu);
        }
        warn!("info: fetched {} flexible gpus", self.flexible_gpus.len());
        Ok(())
    }

    pub fn fill_resource_flexible_gpus(&self, resources: &mut Resources) {
        if self.flexible_gpus.is_empty() && self.need_default_resource {
            resources.resources.push(Resource::FlexibleGpu(FlexibleGpu {
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                ..Default::default()
            }));
        }
        for (flexible_gpu_id, flexible_gpu) in &self.flexible_gpus {
            let Some(model_name) = flexible_gpu.model_name.clone() else {
                warn!("warning: a flexible gpu did not have a model name");
                continue;
            };
            let Some(state) = flexible_gpu.state.clone() else {
                warn!("warning: a flexible gpu did not have a state");
                continue;
            };
            let price_per_hour = match state.as_str() {
                "attached" | "attaching" => self.catalog_entry(
                    "TinaOS-FCU",
                    format!("Gpu:attach:{model_name}").as_str(),
                    "AllocateGpu",
                ),
                "allocated" | "detaching" => self.catalog_entry(
                    "TinaOS-FCU",
                    format!("Gpu:allocate:{model_name}").as_str(),
                    "AllocateGpu",
                ),
                _ => {
                    warn!("warning: a flexible gpus does not have standard state");
                    continue;
                }
            };

            if price_per_hour.is_none() {
                warn!(
                    "{}",
                    format!(
                        "warning: could not retrieve the catalog for {model_name} in state {state}"
                    )
                );
                continue;
            }

            let core_flexible_gpu = FlexibleGpu {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(flexible_gpu_id.clone()),
                price_per_hour,
                price_per_month: None,
                model_name: Some(model_name),
            };
            resources
                .resources
                .push(Resource::FlexibleGpu(core_flexible_gpu));
        }
    }
}
