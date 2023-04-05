use std::error;

use log::{info, warn};
use outscale_api::{
    apis::volume_api::read_volumes,
    models::{FiltersVolume, ReadVolumesRequest, ReadVolumesResponse},
};

pub type VolumeId = String;
const RESOURCE_NAME: &str = "Volume";

use crate::{
    core::{volumes::Volume, Resource, Resources},
    VERSION,
};

use super::Input;

impl Input {
    pub fn fetch_volumes(&mut self) -> Result<(), Box<dyn error::Error>> {
        if self.skip_fetch(RESOURCE_NAME) {
            return Ok(());
        }
        let result: ReadVolumesResponse = loop {
            let filter_volumes: FiltersVolume = match &self.filters {
                Some(filter) => FiltersVolume {
                    tag_keys: Some(filter.tag_keys.clone()),
                    tag_values: Some(filter.tag_values.clone()),
                    tags: Some(filter.tags.clone()),
                    ..Default::default()
                },
                None => FiltersVolume::new(),
            };
            let request = ReadVolumesRequest {
                filters: Some(Box::new(filter_volumes)),
                ..Default::default()
            };
            let response = read_volumes(&self.config, Some(request));

            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let volumes = match result.volumes {
            None => {
                warn!("no volume available");
                return Ok(());
            }
            Some(volumes) => volumes,
        };
        for volume in volumes {
            let volume_id = volume.volume_id.clone().unwrap_or_else(|| String::from(""));
            self.volumes.insert(volume_id, volume);
        }
        info!("fetched {} volumes", self.volumes.len());
        Ok(())
    }

    pub fn fill_resource_volume(&self, resources: &mut Resources) {
        for (volume_id, volume) in &self.volumes {
            let specs = match VolumeSpecs::new(volume, self) {
                Some(s) => s,
                None => continue,
            };
            let core_volume = Volume {
                osc_cost_version: Some(String::from(VERSION)),
                account_id: self.account_id(),
                read_date_rfc3339: self.fetch_date.map(|date| date.to_rfc3339()),
                region: self.region.clone(),
                resource_id: Some(volume_id.clone()),
                price_per_hour: None,
                price_per_month: None,
                volume_type: Some(specs.volume_type.clone()),
                volume_iops: Some(specs.iops),
                volume_size: Some(specs.size),
                price_gb_per_month: specs.price_gb_per_month,
                price_iops_per_month: specs.price_iops_per_month,
            };
            resources.resources.push(Resource::Volume(core_volume));
        }
    }
}

struct VolumeSpecs {
    volume_type: String,
    size: i32,
    iops: i32,
    price_gb_per_month: f32,
    price_iops_per_month: f32,
}

impl VolumeSpecs {
    fn new(volume: &outscale_api::models::Volume, input: &Input) -> Option<Self> {
        let volume_type = match &volume.volume_type {
            Some(volume_type) => volume_type,
            None => {
                warn!("warning: cannot get volume type in volume details");
                return None;
            }
        };

        let iops = volume.iops.unwrap_or_else(|| {
            if volume_type == "io1" {
                warn!("cannot get iops in volume details");
            }
            0
        });

        let size = match &volume.size {
            Some(size) => *size,
            None => {
                warn!("cannot get size in volume details");
                return None;
            }
        };
        let out = VolumeSpecs {
            volume_type: volume_type.clone(),
            iops,
            size,
            price_gb_per_month: 0_f32,
            price_iops_per_month: 0_f32,
        };

        out.parse_volume_prices(input)
    }

    fn parse_volume_prices(mut self, input: &Input) -> Option<VolumeSpecs> {
        let price_gb_per_month = input.catalog_entry(
            "TinaOS-FCU",
            &format!("BSU:VolumeUsage:{}", self.volume_type),
            "CreateVolume",
        )?;
        if self.volume_type == "io1" {
            self.price_iops_per_month = input.catalog_entry(
                "TinaOS-FCU",
                &format!("BSU:VolumeIOPS:{}", self.volume_type),
                "CreateVolume",
            )?;
        }
        self.price_gb_per_month = price_gb_per_month;
        Some(self)
    }
}
