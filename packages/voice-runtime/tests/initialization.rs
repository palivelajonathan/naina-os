use services::ServiceRegistry;
use std::sync::Arc;
use voice_runtime::{VoiceRuntime, VoiceRuntimeConfig, VoiceState};

#[test]
fn test_voice_runtime_instantiation_and_config() {
    let kernel = Arc::new(kernel::Kernel::new(kernel::KernelConfig::default()));
    let runtime = Arc::new(runtime::Runtime::new(runtime::RuntimeConfig, kernel));
    let services = Arc::new(ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = VoiceRuntimeConfig::default();
    assert_eq!(config.sample_rate, 16000);
    assert_eq!(config.channels, 1);
    assert_eq!(config.latency_target_ms, 700);

    let vr = VoiceRuntime::new(config.clone(), runtime, services);
    assert_eq!(vr.config(), &config);
    assert_eq!(vr.state(), VoiceState::Idle);
}
