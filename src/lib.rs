const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod core;
pub mod oapi;

#[macro_export]
macro_rules! choose_default {
    ($resource_type: expr, $default: expr, $data: expr, $need_default_resource: expr) => {
        match $resource_type.is_empty() && $need_default_resource {
            true => $default,
            false => $data,
        }
    };
}
