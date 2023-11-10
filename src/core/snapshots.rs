use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceTrait, HOURS_PER_MONTH};

use crate::VERSION;

#[derive(Serialize, Deserialize, Debug)]
pub struct Snapshot {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub volume_size_gib: Option<i32>,
    pub price_gb_per_month: f32,
}

impl ResourceTrait for Snapshot {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_month = 0_f32;
        // The computation is not accurate as this size is maximally over-estimated.
        price_per_month += (self.volume_size_gib.unwrap() as f32) * self.price_gb_per_month;
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

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            osc_cost_version: Some(String::from(VERSION)),
            account_id: Some("".to_string()),
            read_date_rfc3339: Some("".to_string()),
            region: Some("".to_string()),
            resource_id: None,
            price_per_hour: Some(0.0),
            price_per_month: Some(0.0),
            volume_size_gib: None,
            price_gb_per_month: 0.0,
        }
    }
}
