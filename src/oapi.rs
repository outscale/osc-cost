use std::error;
use outscale_api::apis::configuration_file::ConfigurationFile;
use outscale_api::apis::configuration::Configuration;

pub struct OutscaleApiInput {
    config: Configuration,
}

impl OutscaleApiInput {
    pub fn new(profile_name: String) -> Result<OutscaleApiInput, Box<dyn error::Error>> {
        let config_file = ConfigurationFile::load_default()?;
        Ok(OutscaleApiInput {
            config: config_file.configuration(profile_name)?,
        })
    }
}