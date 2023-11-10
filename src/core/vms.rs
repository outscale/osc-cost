use serde::{Deserialize, Serialize};

use super::{ResourceError, ResourceTrait, HOURS_PER_MONTH};

use crate::VERSION;

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
    pub price_license_per_ram_gb_per_hour: f32,
    pub price_license_per_cpu_per_hour: f32,
    pub price_license_per_vm_per_hour: f32,
    pub license_codes: String,
}

impl ResourceTrait for Vm {
    fn compute(&mut self) -> Result<(), ResourceError> {
        let mut price_per_hour = 0_f32;
        price_per_hour += (self.vm_vcpu as f32) * self.price_vcpu_per_hour;
        price_per_hour += (self.vm_ram_gb as f32) * self.price_ram_gb_per_hour;
        price_per_hour += (self.vm_vcpu as f32) * self.price_license_per_cpu_per_hour;
        price_per_hour += (self.vm_ram_gb as f32) * self.price_license_per_ram_gb_per_hour;
        price_per_hour += self.price_license_per_vm_per_hour;
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

impl Default for Vm {
    fn default() -> Self {
        Self {
            osc_cost_version: Some(String::from(VERSION)),
            account_id: Some("".to_string()),
            read_date_rfc3339: Some("".to_string()),
            region: Some("".to_string()),
            resource_id: None,
            price_per_hour: Some(0.0),
            price_per_month: Some(0.0),
            vm_type: None,
            vm_vcpu_gen: None,
            vm_core_performance: None,
            vm_image: None,
            vm_vcpu: usize::MIN,
            vm_ram_gb: usize::MIN,
            price_vcpu_per_hour: 0.0,
            price_ram_gb_per_hour: 0.0,
            price_box_per_hour: 0.0,
            price_license_per_ram_gb_per_hour: 0.0,
            price_license_per_cpu_per_hour: 0.0,
            price_license_per_vm_per_hour: 0.0,
            license_codes: "".to_string(),
        }
    }
}
