use std::fmt;
use serde::Serialize;
use serde_json;
use crate::debug;

static HOURS_PER_MONTH: f32 = (365_f32 * 24_f32) / 12_f32;

pub struct Resources {
    pub vms: Vec<Vm>,
}

impl Resources {
    pub fn compute(&mut self) -> Result<(), ResourceError> {
        for vm in self.vms.iter_mut() {
            vm.compute()?;
        }
        Ok(())
    }

    pub fn cost_per_hour(&self) -> Result<f32, ResourceError> {
        let mut total = 0f32;
        for vm in &self.vms {
            total += vm.price_per_hour()?
        }
        return Ok(total);
    }

    pub fn cost_per_month(&self) -> Result<f32, ResourceError> {
        Ok(self.cost_per_hour()? * HOURS_PER_MONTH)
    }

    pub fn json(&self) -> serde_json::Result<String> {
        let mut out = String::new();
        for vm in &self.vms {
            match serde_json::to_string(vm) {
                Ok(serialized) => out.push_str(serialized.as_str()),
                Err(e) => {
                    if debug() {
                        eprintln!("warning: provide vm serialization: {}", e);
                    }
                    continue;
                }
            };
            out.push('\n');
        }
        out.pop();
        Ok(out)
    }
}

pub enum ResourceError {
    NotComputed,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResourceError::NotComputed => write!(f, "resource price is not computed yet"),
        }
    }
}

trait Resource {
    fn price_per_hour(&self) -> Result<f32, ResourceError>;
    fn compute(&mut self) -> Result<(), ResourceError>;
}

#[derive(Serialize)]
pub struct Vm {
    pub osc_cost_version: Option<String>,
    pub account_id: Option<String>,
    pub resource_type: Option<String>,
    pub read_date_rfc3339: Option<String>,
    pub region: Option<String>,
    pub resource_id: Option<String>,
    pub price_per_hour: Option<f32>,
    pub price_per_month: Option<f32>,
    pub vm_type: Option<String>,
    pub vm_vcpu_gen: Option<String>,
    pub vm_core_performance: Option<String>,
    pub vm_image: Option<String>,
    pub vm_product_id: Option<String>,
    // Mandatory to compute price for tina types
    pub vm_vcpu: usize,
    pub vm_ram_gb: usize,
    pub price_vcpu_per_hour: f32,
    pub price_ram_gb_per_hour: f32,
    // Mandatory to compute price for BoxUsage (aws-type, etc) types
    pub price_box_per_hour: f32,
    // Mandatory to compute price for all vm types
    pub price_product_per_cpu_per_hour: f32,
}

impl Resource for Vm {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_hour = 0_f32;
        price_per_hour += self.vm_vcpu as f32 * self.price_vcpu_per_hour;
        price_per_hour += self.vm_ram_gb as f32 * self.price_ram_gb_per_hour;
        price_per_hour += self.vm_vcpu as f32 * self.price_product_per_cpu_per_hour;
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
}