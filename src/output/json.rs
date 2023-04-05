use log::warn;
use osc_cost::core::Resources;

pub fn json(resources: &Resources) -> serde_json::Result<String> {
    let mut out = String::new();
    for resource in &resources.resources {
        match serde_json::to_string(resource) {
            Ok(serialized) => out.push_str(serialized.as_str()),
            Err(e) => {
                warn!("provide vm serialization: {}", e);
                continue;
            }
        }
        out.push('\n');
    }
    out.pop();
    Ok(out)
}
