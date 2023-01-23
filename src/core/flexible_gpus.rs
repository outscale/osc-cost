use prometheus::{
    core::{AtomicF64, GenericGauge},
    Gauge, Opts, Registry,
};
use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceMetricsTrait, ResourceTrait, HOURS_PER_MONTH};

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
        self.price_per_month = Some(self.price_per_hour.unwrap() * HOURS_PER_MONTH);
        Ok(())
    }

    fn price_per_hour(&self) -> Result<f32, ResourceError> {
        match self.price_per_hour {
            Some(price) => Ok(price),
            None => Err(ResourceError::NotComputed),
        }
    }
    fn gauge_hour(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let flexible_gpu_gauge_hour_opts =
            Opts::new("flexible_gpu_price_hour", "Flexibe gpu price by hour")
                .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
                .const_label("account_id", self.account_id.as_ref().unwrap())
                .const_label("region", self.region.as_ref().unwrap())
                .const_label("resource_id", self.resource_id.as_ref().unwrap())
                .const_label("resource_type", "FlexibleGpu".to_string());
        let flexible_gpu_gauge_hour =
            Gauge::with_opts(flexible_gpu_gauge_hour_opts).or_else(|e| Err(e));
        flexible_gpu_gauge_hour
    }

    fn gauge_month(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let flexible_gpu_gauge_month_opts =
            Opts::new("flexible_gpu_price_month", "Flexible gpu price by month")
                .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
                .const_label("account_id", self.account_id.as_ref().unwrap())
                .const_label("region", self.region.as_ref().unwrap())
                .const_label("resource_id", self.resource_id.as_ref().unwrap())
                .const_label("resource_type", "FlexibleGpu".to_string());
        let flexible_gpu_gauge_month =
            Gauge::with_opts(flexible_gpu_gauge_month_opts).or_else(|e| Err(e));
        flexible_gpu_gauge_month
    }
}

pub struct FlexibleGpuMetrics {
    pub flexible_gpu_price_per_hours: GenericGauge<AtomicF64>,
    pub flexible_gpu_price_per_months: GenericGauge<AtomicF64>,
}
impl ResourceMetricsTrait for FlexibleGpuMetrics {
    fn register(&self, registry: Registry) -> Result<Registry, prometheus::Error> {
        registry
            .register(Box::new(self.flexible_gpu_price_per_hours.clone()))
            .or_else(|e| Err(e))?;
        registry
            .register(Box::new(self.flexible_gpu_price_per_months.clone()))
            .or_else(|e| Err(e))?;
        Ok(registry)
    }
}
