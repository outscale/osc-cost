use std::error;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::configuration::Configuration;

pub struct OutscaleApiInput {
    config: Configuration,
}

impl OutscaleApiInput {
    pub fn new(profile_name: Option<String>) -> Result<OutscaleApiInput, Box<dyn error::Error>> {
        let config_file = ConfigurationFile::load_default()?;
        let config = match profile_name {
            Some(name) => config_file.configuration(name)?,
            None => config_file.configuration("default")?,
        };
        Ok(OutscaleApiInput {
            config: config,
        })
    }
}