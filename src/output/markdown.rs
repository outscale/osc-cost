use comfy_table::{presets::ASCII_MARKDOWN, Cell, ContentArrangement, Table};
use log::warn;
use osc_cost::core::{digest::Drifts, Resource, Resources};
use std::error::Error;

use super::get_currency;

pub trait Markdown {
    fn markdown(&self) -> Result<String, Box<dyn Error>>;
}

impl Markdown for Resources {
    fn markdown(&self) -> Result<String, Box<dyn Error>> {
        let mut currency: String = String::new();
        let mut account_id: String = String::new();

        let mut table_resource = Table::new();
        table_resource
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(100)
            .set_header(vec![
                "Resource Type",
                "Count",
                "Total price per hour",
                "Total price per month",
                "Total price per year",
            ]);

        for resource in self.resources.iter() {
            match resource {
                Resource::Aggregate(agg) => {
                    if currency.is_empty() {
                        currency =
                            get_currency(agg.region.as_ref().ok_or("could not get the region")?);
                    }
                    if account_id.is_empty() {
                        account_id = agg
                            .account_id
                            .clone()
                            .ok_or("could not get the account_id")?;
                    }
                    table_resource.add_row(vec![
                        agg.aggregated_resource_type.clone(),
                        format!("{}", agg.count),
                        format!(
                            "{:.2}{}",
                            agg.price_per_hour
                                .ok_or("could not get the price_per_hour")?,
                            currency
                        ),
                        format!(
                            "{:.2}{}",
                            agg.price_per_month
                                .ok_or("could not get the price_per_month")?,
                            currency
                        ),
                        format!(
                            "{:.2}{}",
                            agg.price_per_month
                                .ok_or("could not get the price_per_year")?
                                * 12.0,
                            currency
                        ),
                    ])
                }
                _ => {
                    warn!("Got a resource which is not an Aggregate for markdown output");
                    continue;
                }
            };
        }
        let mut table = Table::new();
        table
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(100)
            .add_row(vec![Cell::new("Account Id"), Cell::new(account_id)])
            .add_row(vec![
                Cell::new("Total price per hour"),
                Cell::new(format!("{:.2}{}", self.cost_per_hour()?, currency)),
            ])
            .add_row(vec![
                Cell::new("Total price per month"),
                Cell::new(format!("{:.2}{}", self.cost_per_month()?, currency)),
            ])
            .add_row(vec![
                Cell::new("Total price per year"),
                Cell::new(format!("{:.2}{}", self.cost_per_year()?, currency)),
            ]);

        Ok(format!("Summary:\n{table}\n\nDetails:\n{table_resource}"))
    }
}

impl Markdown for Drifts {
    fn markdown(&self) -> Result<String, Box<dyn Error>> {
        let mut table_resource = Table::new();
        table_resource
            .load_preset(ASCII_MARKDOWN)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(100)
            .set_header(vec!["Resource Type", "Osc-cost", "Digest", "Drift"]);

        for drift in &self.drifts {
            table_resource.add_row(vec![
                drift.category.clone(),
                format!("{:.2}", drift.osc_cost_price),
                format!("{:.2}", drift.digest_price),
                format!("{}%", drift.drift),
            ]);
        }

        Ok(format!("{table_resource}"))
    }
}
