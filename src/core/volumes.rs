use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceTrait, HOURS_PER_MONTH};

use crate::VERSION;

#[derive(Serialize, Deserialize, Debug)]
pub struct Volume {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub volume_type: Option<String>,
    pub volume_size: Option<i32>,
    pub volume_iops: Option<i32>,
    pub price_gb_per_month: f32,
    pub price_iops_per_month: f32,
}

impl ResourceTrait for Volume {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_month = 0_f32;
        price_per_month += (self.volume_size.unwrap() as f32) * self.price_gb_per_month;
        price_per_month += (self.volume_iops.unwrap() as f32) * self.price_iops_per_month;
        self.price_per_hour = Some(price_per_month / HOURS_PER_MONTH);
        self.price_per_month = Some(price_per_month);
        Ok(())
    }

    fn price_per_hour(&self) -> Result<f32, ResourceError> {
        match self.price_per_hour {
            Some(price) => Ok(price),
            None => Err(ResourceError::NotComputed),
        }
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            osc_cost_version: Some(String::from(VERSION)),
            account_id: Some("".to_string()),
            read_date_rfc3339: Some("".to_string()),
            region: Some("".to_string()),
            resource_id: Some("".to_string()),
            price_per_hour: Some(0.0),
            price_per_month: Some(0.0),
            volume_type: Some("".to_string()),
            volume_size: Some(0),
            volume_iops: Some(0),
            price_gb_per_month: 0.0,
            price_iops_per_month: 0.0,
        }
    }
}
