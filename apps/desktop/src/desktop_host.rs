//! Composition root application manager for NAINA OS Desktop Host.

use crate::config::DesktopHostConfig;
use crate::error::{DesktopHostError, DesktopHostResult};
use crate::lifecycle::SignalWatcher;
use crate::types::DesktopHostState;
use automation::{AutomationConfig, AutomationEngine};
use browser_runtime::{BrowserRuntime, BrowserRuntimeConfig};
use context_engine::{ContextEngine, ContextEngineConfig};
use desktop_runtime::{DesktopRuntime, DesktopRuntimeConfig};
use logging::{LogLevel, Logger, LoggerConfig};
use memory::{MemoryConfig, MemoryStore};
use model_runtime::ModelRuntime;
use orchestrator::{Orchestrator, TaskRequest};
use runtime::Runtime;
use sdk::{SDKBuilder, SDKFacade};
use services::ServiceRegistry;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use tool_registry::{ToolRegistry, ToolRegistryConfig};
use ui::{UIBuilder, UIFramework};
use voice_runtime::{AudioBuffer, VoiceRuntime, VoiceRuntimeConfig};

/// Composition root application managing the NAINA OS Desktop Host lifecycle and cognitive loop.
pub struct DesktopHostApp {
    config: DesktopHostConfig,
    sdk: Arc<SDKFacade>,
    ui: Arc<UIFramework>,
    orchestrator: Arc<Orchestrator>,
    voice: Arc<VoiceRuntime>,
    desktop: Arc<DesktopRuntime>,
    browser: Arc<BrowserRuntime>,
    automation: Arc<AutomationEngine>,
    logger: Mutex<Logger>,
    state: RwLock<DesktopHostState>,
    signal_watcher: SignalWatcher,
}

unsafe impl Send for DesktopHostApp {}
unsafe impl Sync for DesktopHostApp {}

impl fmt::Debug for DesktopHostApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopHostApp")
            .field("config", &self.config)
            .field("state", &self.state)
            .field("signal_watcher", &self.signal_watcher)
            .finish()
    }
}

impl DesktopHostApp {
    /// Boots and composes the [`DesktopHostApp`] instance with pre-configured runtime handles.
    pub fn boot(
        config: DesktopHostConfig,
        runtime: Arc<Runtime>,
        services: Arc<ServiceRegistry>,
    ) -> DesktopHostResult<Self> {
        let logger = Logger::new(LoggerConfig::default())?.with_component("desktop-host");

        let _ = logger.log(
            LogLevel::Info,
            "Booting NAINA OS Desktop Host composition root...".to_string(),
        );

        // 1. SDK Facade
        let sdk = Arc::new(
            SDKBuilder::new()
                .with_runtime(Arc::clone(&runtime))
                .with_services(Arc::clone(&services))
                .build()?,
        );

        // 2. UI Framework
        let ui = Arc::new(
            UIBuilder::new()
                .with_runtime(Arc::clone(&runtime))
                .build()?,
        );
        ui.initialize()?;

        // 3. Core Engine Runtimes
        let memory = Arc::new(MemoryStore::new(MemoryConfig::default()));

        let context_engine = Arc::new(ContextEngine::new(
            ContextEngineConfig::default(),
            Arc::clone(&memory),
        ));

        let tool_registry = Arc::new(ToolRegistry::new(ToolRegistryConfig, Arc::clone(&runtime)));

        let model_runtime = Arc::new(ModelRuntime::new(
            model_runtime::ModelRuntimeConfig::default(),
        ));

        let orchestrator = Arc::new(Orchestrator::new(
            orchestrator::OrchestratorConfig::default(),
            Arc::clone(&runtime),
            Arc::clone(&memory),
            Arc::clone(&context_engine),
            Arc::clone(&model_runtime),
            Arc::clone(&tool_registry),
        ));

        // 4. Subsystem Runtimes
        let voice = Arc::new(VoiceRuntime::new(
            VoiceRuntimeConfig::default(),
            Arc::clone(&runtime),
            Arc::clone(&services),
        ));

        let desktop = Arc::new(DesktopRuntime::new(
            DesktopRuntimeConfig::default(),
            Arc::clone(&runtime),
            Arc::clone(&services),
        ));

        let browser = Arc::new(BrowserRuntime::new(
            BrowserRuntimeConfig::default(),
            Arc::clone(&runtime),
            Arc::clone(&services),
        ));

        let automation = Arc::new(AutomationEngine::new(
            AutomationConfig::default(),
            Arc::clone(&runtime),
            Arc::clone(&services),
        ));

        // 5. Register Subsystem Services into ServiceRegistry
        let _ = services.register("service.voice", None, None);
        let _ = services.register("service.model", None, None);
        let _ = services.register("service.memory", None, None);
        let _ = services.register("service.tools", None, None);
        let _ = services.register("service.desktop", None, None);
        let _ = services.register("service.browser", None, None);
        let _ = services.register("service.automation", None, None);
        let _ = services.register("service.orchestration", None, None);
        let _ = services.register("service.ui", None, None);

        let app = Self {
            config,
            sdk,
            ui,
            orchestrator,
            voice,
            desktop,
            browser,
            automation,
            logger: Mutex::new(logger),
            state: RwLock::new(DesktopHostState::Ready),
            signal_watcher: SignalWatcher::new(),
        };

        if let Ok(logger) = app.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                "NAINA OS Desktop Host booted successfully and state is Ready".to_string(),
            );
        }

        Ok(app)
    }

    /// Returns the current lifecycle state of the Desktop Host application.
    pub fn state(&self) -> DesktopHostState {
        self.state
            .read()
            .map(|s| *s)
            .unwrap_or(DesktopHostState::Stopped)
    }

    /// Returns a reference to the application configuration.
    pub fn config(&self) -> &DesktopHostConfig {
        &self.config
    }

    /// Returns a reference to the active signal watcher.
    pub fn signal_watcher(&self) -> &SignalWatcher {
        &self.signal_watcher
    }

    /// Returns a reference to the composed SDK facade.
    pub fn sdk(&self) -> &Arc<SDKFacade> {
        &self.sdk
    }

    /// Returns a reference to the composed UI framework.
    pub fn ui(&self) -> &Arc<UIFramework> {
        &self.ui
    }

    /// Returns a reference to the composed orchestrator.
    pub fn orchestrator(&self) -> &Arc<Orchestrator> {
        &self.orchestrator
    }

    /// Returns a reference to the composed voice runtime.
    pub fn voice(&self) -> &Arc<VoiceRuntime> {
        &self.voice
    }

    /// Returns a reference to the composed desktop runtime.
    pub fn desktop(&self) -> &Arc<DesktopRuntime> {
        &self.desktop
    }

    /// Returns a reference to the composed browser runtime.
    pub fn browser(&self) -> &Arc<BrowserRuntime> {
        &self.browser
    }

    /// Returns a reference to the composed automation engine.
    pub fn automation(&self) -> &Arc<AutomationEngine> {
        &self.automation
    }

    /// Executes the Minimum Viable NAINA (MVN) cognitive turn pipeline.
    pub fn process_voice_turn(&self, pcm_audio: &[u8]) -> DesktopHostResult<Vec<u8>> {
        let mut state_lock = self
            .state
            .write()
            .map_err(|_| DesktopHostError::LockError {
                message: "Failed to acquire state write lock".to_string(),
            })?;

        if *state_lock != DesktopHostState::Ready {
            return Err(DesktopHostError::BootFailed {
                message: format!("Cannot process turn in state {:?}", *state_lock),
            });
        }

        *state_lock = DesktopHostState::ProcessingTurn;

        // 1. Voice STT Transcription
        let audio_buf = AudioBuffer::new(16000, 1, pcm_audio.to_vec());
        let transcription_res = self.voice.process_audio_input(&audio_buf)?;
        let transcription = transcription_res.text;

        // 2. Orchestration & Task Execution
        let prompt = if transcription.trim().is_empty() {
            "Explain NAINA OS status".to_string()
        } else {
            transcription
        };

        let task_req = TaskRequest {
            task_id: format!("task-mvn-{}", pcm_audio.len()),
            prompt,
            conversation_id: None,
        };

        let task_resp = self.orchestrator.execute_task(task_req)?;

        // 3. Voice TTS Synthesis
        let synthesis_text = if task_resp.output.is_empty() {
            "NAINA OS MVN Turn completed successfully".to_string()
        } else {
            task_resp.output
        };

        let synthesis_res = self.voice.synthesize_speech(&synthesis_text)?;
        let audio_output = synthesis_res.audio.pcm_data;

        *state_lock = DesktopHostState::Ready;

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                format!(
                    "MVN turn processed cleanly with {} output audio bytes",
                    audio_output.len()
                ),
            );
        }

        Ok(audio_output)
    }

    /// Shuts down the Desktop Host application and all composed subsystem runtimes.
    pub fn shutdown(&self) -> DesktopHostResult<()> {
        let mut state_lock = self
            .state
            .write()
            .map_err(|_| DesktopHostError::LockError {
                message: "Failed to acquire state write lock".to_string(),
            })?;

        if *state_lock == DesktopHostState::Stopped {
            return Ok(());
        }

        *state_lock = DesktopHostState::ShuttingDown;
        self.signal_watcher.request_shutdown();

        // Subsystem Shutdowns
        self.voice.handle_barge_in();
        self.browser.cancel_operation();
        self.automation.cancel();
        let _ = self.ui.shutdown();
        let _ = self.sdk.shutdown();

        *state_lock = DesktopHostState::Stopped;

        if let Ok(logger) = self.logger.lock() {
            let _ = logger.log(
                LogLevel::Info,
                "NAINA OS Desktop Host composition root shut down cleanly".to_string(),
            );
        }

        Ok(())
    }
}
