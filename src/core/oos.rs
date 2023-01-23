
use prometheus::{
    core::{AtomicF64, GenericGauge},
    Gauge, Opts, Registry,
};
use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceMetricsTrait, ResourceTrait, HOURS_PER_MONTH};

#[derive(Serialize, Deserialize, Debug)]
pub struct Oos {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub size_gb: Option<f32>,
    pub price_gb_per_month: f32,
}

impl ResourceTrait for Oos {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_month = 0_f32;
        price_per_month += (self.size_gb.unwrap() as f32) * self.price_gb_per_month;
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
    fn gauge_hour(
        &self,
    ) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let oos_gauge_hour_opts = Opts::new("oos_price_hour", "Oos price by hour")
            .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
            .const_label("account_id", self.account_id.as_ref().unwrap())
            .const_label("region", self.region.as_ref().unwrap())
            .const_label("resource_id", self.resource_id.as_ref().unwrap())
            .const_label("resource_type", "Oos".to_string());
        let oos_gauge_hour = Gauge::with_opts(oos_gauge_hour_opts).or_else(|e| Err(e));
        oos_gauge_hour
    }

    fn gauge_month(
        &self,
    ) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let oos_gauge_month_opts = Opts::new("oos_price_month", "Oos price by month")
            .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
            .const_label("account_id", self.account_id.as_ref().unwrap())
            .const_label("region", self.region.as_ref().unwrap())
            .const_label("resource_id", self.resource_id.as_ref().unwrap())
            .const_label("resource_type", "Oos".to_string());
        let oos_gauge_month = Gauge::with_opts(oos_gauge_month_opts).or_else(|e| Err(e));
        oos_gauge_month
    }
}

pub struct OosMetrics {
    pub oos_price_per_hours: GenericGauge<AtomicF64>,
    pub oos_price_per_months: GenericGauge<AtomicF64>,
}
impl ResourceMetricsTrait for OosMetrics {
    fn register(&self, registry: Registry) -> Result<Registry, prometheus::Error> {
        registry
            .register(Box::new(self.oos_price_per_hours.clone()))
            .or_else(|e| Err(e))?;
        registry
            .register(Box::new(self.oos_price_per_months.clone()))
            .or_else(|e| Err(e))?;
        Ok(registry)
    }

}
