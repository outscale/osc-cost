use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceTrait, HOURS_PER_MONTH};

#[derive(Serialize, Deserialize, Debug)]
pub struct PublicIp {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub price_non_attached: Option<f32>,
    pub price_first_ip: Option<f32>,
    pub price_next_ips: Option<f32>,
}

impl ResourceTrait for PublicIp {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_hour: f32 = 0.0;
        if let Some(price_non_attached) = self.price_non_attached {
            price_per_hour += price_non_attached;
        } else if let Some(price_first_ip) = self.price_first_ip {
            price_per_hour += price_first_ip;
        } else if let Some(price_next_ips) = self.price_next_ips {
            price_per_hour += price_next_ips;
        }
        self.price_per_hour = Some(price_per_hour);
        self.price_per_month = Some(price_per_hour * HOURS_PER_MONTH);
        Ok(())
    }

    fn price_per_hour(&self) -> Result<f32, ResourceError> {
        match self.price_per_hour {
            Some(price) => Ok(price),
            None => Err(ResourceError::NotComputed),
        }
    }
}
