use osc_cost::core::Resources;

mod error;
mod ser;

pub fn ods(resources: &Resources) -> error::Result<Vec<u8>> {
    ser::to_bytes(&resources.resources)
}
