use crate::models::Config;
use crate::traits::ConfigProvider;
use crate::types::Environment;

impl ConfigProvider for Config {
    fn get_config(&self) -> &Config {
        self
    }

    fn environment(&self) -> Environment {
        self.environment
    }
}
