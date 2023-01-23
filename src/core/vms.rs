use super::{ResourceError, ResourceMetricsTrait, ResourceTrait, HOURS_PER_MONTH};
use prometheus::{
    core::{AtomicF64, GenericGauge},
    Gauge, Opts, Registry,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Vm {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub vm_type: Option<String>,
    pub vm_vcpu_gen: Option<String>,
    pub vm_core_performance: Option<String>,
    pub vm_image: Option<String>,
    // Mandatory to compute price for tina types
    pub vm_vcpu: usize,
    pub vm_ram_gb: usize,
    pub price_vcpu_per_hour: f32,
    pub price_ram_gb_per_hour: f32,
    // Mandatory to compute price for BoxUsage (aws-type, etc) types
    pub price_box_per_hour: f32,
    // Mandatory to compute price for all vm types
    pub price_product_per_ram_gb_per_hour: f32,
    pub price_product_per_cpu_per_hour: f32,
    pub price_product_per_vm_per_hour: f32,
}

impl ResourceTrait for Vm {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_hour = 0_f32;
        price_per_hour += (self.vm_vcpu as f32) * self.price_vcpu_per_hour;
        price_per_hour += (self.vm_ram_gb as f32) * self.price_ram_gb_per_hour;
        price_per_hour += (self.vm_vcpu as f32) * self.price_product_per_cpu_per_hour;
        price_per_hour += (self.vm_ram_gb as f32) * self.price_product_per_ram_gb_per_hour;
        price_per_hour += self.price_product_per_vm_per_hour;
        price_per_hour += self.price_box_per_hour;
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
        let vm_gauge_hour_opts = Opts::new("vm_price_hour", "Vm price by hour")
            .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
            .const_label("account_id", self.account_id.as_ref().unwrap())
            .const_label("region", self.region.as_ref().unwrap())
            .const_label("resource_id", self.resource_id.as_ref().unwrap())
            .const_label("resource_type", "Vm".to_string());
        let vm_gauge_hour = Gauge::with_opts(vm_gauge_hour_opts).or_else(|e| Err(e));
        vm_gauge_hour
    }

    fn gauge_month(&self) -> Result<GenericGauge<AtomicF64>, prometheus::Error> {
        let vm_gauge_month_opts = Opts::new("vm_price_month", "Vm price by month")
            .const_label("osc_cost_version", self.osc_cost_version.as_ref().unwrap())
            .const_label("account_id", self.account_id.as_ref().unwrap())
            .const_label("region", self.region.as_ref().unwrap())
            .const_label("resource_id", self.resource_id.as_ref().unwrap())
            .const_label("resource_type", "Vm".to_string());
        let vm_gauge_month = Gauge::with_opts(vm_gauge_month_opts).or_else(|e| Err(e));
        vm_gauge_month
    }
}

pub struct VmMetrics {
    pub vm_price_per_hours: GenericGauge<AtomicF64>,
    pub vm_price_per_months: GenericGauge<AtomicF64>,
}
impl ResourceMetricsTrait for VmMetrics {
    fn register(&self, registry: Registry) -> Result<Registry, prometheus::Error> {
        registry
            .register(Box::new(self.vm_price_per_hours.clone()))
            .or_else(|e| Err(e))?;
        registry
            .register(Box::new(self.vm_price_per_months.clone()))
            .or_else(|e| Err(e))?;
        Ok(registry)
    }
}
