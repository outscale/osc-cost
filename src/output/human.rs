use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, ContentArrangement, Table,
};
use log::warn;
use osc_cost::core::{Resource, Resources};
use std::error::Error;

use super::get_currency;

pub fn human(resources: &Resources) -> Result<String, Box<dyn Error>> {
    let mut currency: String = String::new();
    let mut account_id: String = String::new();

    let mut table_resource = Table::new();
    table_resource
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(100)
        .set_header(vec![
            "Resource Type",
            "Count",
            "Total price per hour",
            "Total price per month",
            "Total price per year",
        ]);

    for resource in resources.resources.iter() {
        match resource {
            Resource::Aggregate(agg) => {
                if currency.is_empty() {
                    currency = get_currency(agg.region.as_ref().ok_or("could not get the region")?);
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
                        "{}{}",
                        agg.price_per_hour
                            .ok_or("could not get the price_per_hour")?,
                        currency
                    ),
                    format!(
                        "{}{}",
                        agg.price_per_month
                            .ok_or("could not get the price_per_month")?,
                        currency
                    ),
                    format!(
                        "{}{}",
                        agg.price_per_month
                            .ok_or("could not get the price_per_year")?
                            * 12.0,
                        currency
                    ),
                ])
            }
            _ => {
                warn!("Got a resource which is not an Aggregate for human output");
                continue;
            }
        };
    }
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(100)
        .add_row(vec![Cell::new("Account Id"), Cell::new(account_id)])
        .add_row(vec![
            Cell::new("Total price per hour"),
            Cell::new(format!("{}{}", resources.cost_per_hour()?, currency)),
        ])
        .add_row(vec![
            Cell::new("Total price per month"),
            Cell::new(format!("{}{}", resources.cost_per_month()?, currency)),
        ])
        .add_row(vec![
            Cell::new("Total price per year"),
            Cell::new(format!("{}{}", resources.cost_per_year()?, currency)),
        ]);

    Ok(format!("Summary:\n{table}\n\nDetails:\n{table_resource}"))
}
