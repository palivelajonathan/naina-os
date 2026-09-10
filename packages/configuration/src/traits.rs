use crate::models::Config;
use crate::types::Environment;

/// Public interface for accessing loaded system configuration in NAINA OS.
pub trait ConfigProvider: Send + Sync {
    /// Returns a reference to the root `Config`.
    fn get_config(&self) -> &Config;

    /// Returns the current active `Environment`.
    fn environment(&self) -> Environment {
        self.get_config().environment
    }

    /// Helper indicating whether the system is running in development mode.
    fn is_development(&self) -> bool {
        self.environment() == Environment::Development
    }

    /// Helper indicating whether the system is running in production mode.
    fn is_production(&self) -> bool {
        self.environment() == Environment::Production
    }
}
