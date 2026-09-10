//! Builder pattern for initializing the client SDK facade.

use crate::config::SDKConfig;
use crate::error::{SDKError, SDKResult};
use crate::sdk_facade::SDKFacade;
use logging::{Logger, LoggerConfig};
use runtime::Runtime;
use services::ServiceRegistry;
use std::sync::Arc;

/// Builder for constructing and configuring an [`SDKFacade`].
#[derive(Debug, Default)]
pub struct SDKBuilder {
    config: Option<SDKConfig>,
    runtime: Option<Arc<Runtime>>,
    services: Option<Arc<ServiceRegistry>>,
}

impl SDKBuilder {
    /// Constructs a new default [`SDKBuilder`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets custom [`SDKConfig`] options for the SDK instance.
    pub fn with_config(mut self, config: SDKConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Supplies an existing [`Runtime`] instance.
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Supplies an existing [`ServiceRegistry`] instance.
    pub fn with_services(mut self, services: Arc<ServiceRegistry>) -> Self {
        self.services = Some(services);
        self
    }

    /// Builds and initializes the microkernel [`SDKFacade`].
    pub fn build(self) -> SDKResult<SDKFacade> {
        let config = self.config.unwrap_or_default();

        // Logger
        let logger = Logger::new(LoggerConfig::default())?.with_component("sdk");

        let runtime = self.runtime.ok_or_else(|| SDKError::InitializationFailed {
            message: "Runtime handle must be provided to SDKBuilder via with_runtime()".to_string(),
        })?;

        if config.auto_start_runtime {
            let _ = runtime.initialize(None);
            let _ = runtime.start(None);
        }

        let services = self.services.unwrap_or_else(|| {
            Arc::new(ServiceRegistry::new(
                services::ServicesConfig,
                Arc::clone(&runtime),
            ))
        });

        let facade = SDKFacade::new(config, runtime, services, logger);
        Ok(facade)
    }
}
