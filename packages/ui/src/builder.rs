//! Builder pattern for constructing and configuring the UIFramework.

use crate::config::UIConfig;
use crate::error::{UIError, UIResult};
use crate::ui_framework::UIFramework;
use logging::{Logger, LoggerConfig};
use runtime::Runtime;
use std::sync::Arc;

/// Builder for configuring and instantiating a [`UIFramework`].
#[derive(Debug, Default)]
pub struct UIBuilder {
    config: Option<UIConfig>,
    runtime: Option<Arc<Runtime>>,
}

impl UIBuilder {
    /// Constructs a new [`UIBuilder`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets custom [`UIConfig`] options for the UI framework instance.
    pub fn with_config(mut self, config: UIConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Supplies the microkernel [`Runtime`] handle.
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Builds and constructs the [`UIFramework`].
    pub fn build(self) -> UIResult<UIFramework> {
        let config = self.config.unwrap_or_default();
        let logger = Logger::new(LoggerConfig::default())?.with_component("ui");

        let runtime = self.runtime.ok_or_else(|| UIError::InitializationFailed {
            message: "Runtime handle must be provided to UIBuilder via with_runtime()".to_string(),
        })?;

        let framework = UIFramework::new(config, runtime, logger);
        Ok(framework)
    }
}
