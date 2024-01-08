use osc_cost::core::Resources;

use self::ser::{to_prom, CustomLabelKey};

mod error;
mod ser;

pub fn prometheus(resources: &Resources) -> error::Result<String> {
    let keep_label = vec![
        "account_id".to_string(),
        "osc_cost_version".to_string(),
        "region".to_string(),
        "resource_type".to_string(),
        "resource_id".to_string(),
        "price_per_hour".to_string(),
        "price_per_month".to_string(),
        "nested_virtualization".to_string(),
        "tenancy".to_string(),
    ];

    let primary_name = "_price_hour".to_string();
    let primary_help = " price by hour".to_string();
    let primary_label_key = "price_per_hour".to_string();
    let secondary_name = "_price_month".to_string();
    let secondary_help = " price by month".to_string();
    let secondary_label_key = "price_per_month".to_string();
    let label_type = "resource_id".to_string();
    let primary = CustomLabelKey {
        name: primary_name,
        help: primary_help,
        key: primary_label_key,
    };
    let secondary = CustomLabelKey {
        name: secondary_name,
        help: secondary_help,
        key: secondary_label_key,
    };

    to_prom(
        &resources.resources,
        keep_label,
        primary,
        secondary,
        label_type,
    )
}
