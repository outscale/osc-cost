use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceTrait, HOURS_PER_MONTH};

use crate::VERSION;

#[derive(Serialize, Deserialize, Debug)]
pub struct NatServices {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_product_per_nat_service_per_hour: Option<f32>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
}

impl ResourceTrait for NatServices {
    fn price_per_hour(&self) -> Result<f32, ResourceError> {
        match self.price_per_hour {
            Some(price) => Ok(price),
            None => Err(ResourceError::NotComputed),
        }
    }
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_hour = 0_f32;
        if let Some(price_non_attached) = self.price_product_per_nat_service_per_hour {
            price_per_hour += price_non_attached;
        }
        self.price_per_hour = Some(price_per_hour);
        self.price_per_month = Some(price_per_hour * HOURS_PER_MONTH);
        Ok(())
    }
}

impl Default for NatServices {
    fn default() -> Self {
        Self {
            osc_cost_version: Some(String::from(VERSION)),
            account_id: Some("".to_string()),
            read_date_rfc3339: Some("".to_string()),
            region: Some("".to_string()),
            resource_id: None,
            price_per_hour: Some(0.0),
            price_per_month: Some(0.0),
            price_product_per_nat_service_per_hour: Some(0.0),
        }
    }
}
