use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceTrait, HOURS_PER_MONTH};

use crate::VERSION;
#[derive(Serialize, Deserialize, Debug)]
pub struct FlexibleGpu {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub model_name: Option<String>,
}

impl ResourceTrait for FlexibleGpu {
    fn compute(&mut self) -> Result<(), ResourceError> {
        self.price_per_month = Some(self.price_per_hour.unwrap_or_default() * HOURS_PER_MONTH);
        Ok(())
    }

    fn price_per_hour(&self) -> Result<f32, ResourceError> {
        match self.price_per_hour {
            Some(price) => Ok(price),
            None => Err(ResourceError::NotComputed),
        }
    }
}

impl Default for FlexibleGpu {
    fn default() -> Self {
        Self {
            osc_cost_version: Some(String::from(VERSION)),
            account_id: Some("".to_string()),
            read_date_rfc3339: Some("".to_string()),
            region: Some("".to_string()),
            resource_id: None,
            price_per_hour: Some(0.0),
            price_per_month: Some(0.0),
            model_name: None,
        }
    }
}
