use log::warn;
use osc_cost::core::{digest::Drifts, Resources};

pub trait Json {
    fn json(&self) -> serde_json::Result<String>;
}

impl Json for Resources {
    fn json(&self) -> serde_json::Result<String> {
        let mut out = String::new();
        for resource in &self.resources {
            match serde_json::to_string(resource) {
                Ok(serialized) => out.push_str(serialized.as_str()),
                Err(e) => {
                    warn!("resource serialization error: {}", e);
                    continue;
                }
            }
            out.push('\n');
        }
        out.pop();
        Ok(out)
    }
}

impl Json for Drifts {
    fn json(&self) -> serde_json::Result<String> {
        let mut out = String::new();
        for drift in &self.drifts {
            match serde_json::to_string(drift) {
                Ok(serialized) => out.push_str(serialized.as_str()),
                Err(e) => {
                    warn!("drift serialization error: {}", e);
                    continue;
                }
            }
            out.push('\n');
        }
        out.pop();
        Ok(out)
    }
}
