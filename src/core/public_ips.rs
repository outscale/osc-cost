use std::process;

use prometheus::{
    core::{AtomicF64, GenericGauge},
    Gauge, Opts, Registry,
};
use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceMetricsTrait, ResourceTrait, HOURS_PER_MONTH};

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
    fn gauge_hour(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let public_ip_gauge_hour_opts =
            Opts::new("public_ip_price_hour", "Public Ip price by hour")
                .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
                .const_label("account_id", self.account_id.as_ref().unwrap())
                .const_label("region", self.region.as_ref().unwrap())
                .const_label("resource_id", self.resource_id.as_ref().unwrap())
                .const_label("resource_type", "PublicIp".to_string());
        let public_ip_gauge_hour =
            Gauge::with_opts(public_ip_gauge_hour_opts).unwrap_or_else(|err| {
                println!("Error public ip gauge hour: {}", err);
                process::exit(1);
            });
        Ok(public_ip_gauge_hour)
    }

    fn gauge_month(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let public_ip_gauge_month_opts =
            Opts::new("public_ip_price_month", "Public Ip price by month")
                .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
                .const_label("account_id", self.account_id.as_ref().unwrap())
                .const_label("region", self.region.as_ref().unwrap())
                .const_label("resource_id", self.resource_id.as_ref().unwrap())
                .const_label("resource_type", "PublicIp".to_string());
        let public_ip_gauge_month =
            Gauge::with_opts(public_ip_gauge_month_opts).unwrap_or_else(|err| {
                println!("Error public ip gauge month: {}", err);
                process::exit(1)
            });
        Ok(public_ip_gauge_month)
    }
}

pub struct PublicIpMetrics {
    pub publicip_price_per_hours: GenericGauge<AtomicF64>,
    pub publicip_price_per_months: GenericGauge<AtomicF64>,
}
impl ResourceMetricsTrait for PublicIpMetrics {
    fn register(&self, registry: Registry) -> Result<Registry, prometheus::Error> {
        registry
            .register(Box::new(self.publicip_price_per_hours.clone()))
            .or_else(|e| Err(e))?;
        registry
            .register(Box::new(self.publicip_price_per_months.clone()))
            .or_else(|e| Err(e))?;
        Ok(registry)
    }
}
