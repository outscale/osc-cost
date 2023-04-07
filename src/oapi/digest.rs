use std::{collections::HashMap, error};

use lazy_static::lazy_static;
use log::{info, warn};
use outscale_api::{
    apis::account_api::read_consumption_account,
    models::{ConsumptionEntry, ReadConsumptionAccountRequest, ReadConsumptionAccountResponse},
};
use regex::Regex;

use crate::{
    core::digest::{match_entry_id_resource_type, Digest},
    oapi::vms::VmSpecs,
};

use super::Input;
impl Input {
    pub fn fetch_digest(
        &mut self,
        from_date: &str,
        to_date: &str,
    ) -> Result<(), Box<dyn error::Error>> {
        let result: ReadConsumptionAccountResponse = loop {
            let request =
                ReadConsumptionAccountRequest::new(from_date.to_owned(), to_date.to_owned());
            let response = read_consumption_account(&self.config, Some(request));
            if Input::is_throttled(&response) {
                self.random_wait();
                continue;
            }
            break response?;
        };

        let entries = match result.consumption_entries {
            Some(catalog) => catalog,
            None => {
                warn!("no consumption provided");
                return Ok(());
            }
        };

        for entry in entries {
            let service = match &entry.service {
                Some(t) => t.clone(),
                None => {
                    warn!("digest entry has no service");
                    continue;
                }
            };

            let operation = match &entry.operation {
                Some(t) => t.clone(),
                None => {
                    warn!("digest entry has no operation");
                    continue;
                }
            };

            let _type = match &entry._type {
                Some(t) => t.clone(),
                None => {
                    warn!("digest entry has no operation");
                    continue;
                }
            };

            let entry_id = format!("{service}/{_type}/{operation}");
            match self.consumption.get(&entry_id) {
                Some(e) => {
                    self.consumption.insert(
                        entry_id,
                        ConsumptionEntry {
                            account_id: entry.account_id,
                            category: entry.category,
                            from_date: entry.from_date,
                            operation: entry.operation,
                            paying_account_id: entry.paying_account_id,
                            service: entry.service,
                            subregion_name: None,
                            title: entry.title,
                            to_date: entry.to_date,
                            _type: entry._type,
                            value: Some(entry.value.unwrap_or(0.0) + e.value.unwrap_or(0.0)),
                        },
                    );
                }
                None => {
                    self.consumption.insert(entry_id, entry);
                }
            }
        }

        info!("fetched {} consumption entries", self.consumption.len());
        Ok(())
    }

    pub fn fill_digest(&self, digests: &mut HashMap<String, Digest>) {
        for (id, entry) in &self.consumption {
            match id {
                s if s.starts_with("TinaOS-FCU/ProductUsage") => {
                    lazy_static! {
                        static ref REG: Regex =
                            Regex::new(r"^TinaOS-FCU/ProductUsage:(.+)/RunInstances-(\d+)-OD")
                                .unwrap();
                    }
                    let cap = match REG.captures_iter(s).next() {
                        Some(cap) => cap,
                        None => {
                            warn!("Cannot extract tina type {}", s);
                            continue;
                        }
                    };

                    let vm_type = String::from(&cap[1]);
                    let product_code = String::from(&cap[2]);

                    // Extract cores
                    let cores: f32 = if vm_type.starts_with("tina") {
                        match VmSpecs::parse_tina_type(&vm_type) {
                            None => {
                                warn!("Cannot extract cores from tina type");
                                continue;
                            }
                            Some((_, c, _, _)) => c,
                        }
                    } else {
                        match VmSpecs::parse_box_type(&vm_type, self) {
                            None => {
                                warn!("Cannot extract cores from aws type");
                                continue;
                            }
                            Some((_, c, _, _)) => c,
                        }
                    };

                    // Extract product codes
                    let Some(price_factor) = VmSpecs::compute_product_price_per_hour(cores, &product_code) else {
                        warn!("Cannot extract price factor from product codes");
                        continue;
                    };

                    // CustomRam
                    let Some(product_usage_catalog) = self.catalog.get(&format!("TinaOS-FCU/ProductUsage/RunInstances-{}-OD", product_code)) else {
                        warn!("Cannot get product code entry");
                        continue;
                    };

                    let price = price_factor
                        * entry.value.unwrap_or(0.0) as f32
                        * product_usage_catalog.unit_price.unwrap_or(0.0);

                    let category = String::from("Vm");
                    match digests.get(&category) {
                        None => {
                            digests.insert(category, Digest { price: Some(price) });
                        }
                        Some(d) => {
                            digests.insert(
                                category,
                                Digest {
                                    price: Some(d.price.unwrap_or(0.0) + price),
                                },
                            );
                        }
                    }
                }
                s if s.starts_with("TinaOS-FCU/BoxUsage:tina") => {
                    // Convert into CustomCore and CustomRam
                    lazy_static! {
                        static ref REG: Regex = Regex::new(r":(.+)/").unwrap();
                    }
                    let cap = match REG.captures_iter(s).next() {
                        Some(cap) => cap,
                        None => {
                            warn!("Cannot extract tina type {}", s);
                            continue;
                        }
                    };

                    let tina_type = String::from(&cap[1]);
                    let Some((generation, vcpu, ram_gb, performance)) =  VmSpecs::parse_tina_type(&tina_type) else {
                        warn!("Cannot extract value from tina type {}", s);
                            continue
                        };

                    // CustomRam
                    let Some(custom_ram_catalog) = self.catalog.get("TinaOS-FCU/CustomRam/RunInstances-OD") else {
                        warn!("Cannot get customRam entry");
                        continue;
                    };

                    // CustomCore
                    let Some(custom_core_catalog) = self.catalog.get(&format!("TinaOS-FCU/CustomCore:v{}-p{}/RunInstances-OD",generation, performance)) else {
                        warn!("Cannot get customCore entry");
                        continue;
                    };

                    // value *(CustomRam price * ram + CustomCore price * core)
                    let price = ram_gb
                        * entry.value.unwrap_or(0.0) as f32
                        * custom_ram_catalog.unit_price.unwrap_or(0.0)
                        + vcpu
                            * entry.value.unwrap_or(0.0) as f32
                            * custom_core_catalog.unit_price.unwrap_or(0.0);

                    let category = String::from("Vm");
                    match digests.get(&category) {
                        None => {
                            digests.insert(category, Digest { price: Some(price) });
                        }
                        Some(d) => {
                            digests.insert(
                                category,
                                Digest {
                                    price: Some(d.price.unwrap_or(0.0) + price),
                                },
                            );
                        }
                    }
                }
                s => {
                    // CustomCore
                    let Some(catalog_entry) = self.catalog.get(s) else {
                        warn!("Cannot get catalog entry");
                        continue;
                    };

                    let Some(category) = match_entry_id_resource_type(s) else {
                        warn!("Skip {} because if has no category", s);
                        continue;
                    };

                    let price =
                        entry.value.unwrap_or(0.0) as f32 * catalog_entry.unit_price.unwrap_or(0.0);

                    match digests.get(&category) {
                        None => {
                            digests.insert(category, Digest { price: Some(price) });
                        }
                        Some(d) => {
                            digests.insert(
                                category,
                                Digest {
                                    price: Some(d.price.unwrap_or(0.0) + price),
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}
