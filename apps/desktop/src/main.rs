//! Binary composition root executable entry point for NAINA OS Desktop Host.

use desktop_host::{DesktopHostApp, DesktopHostConfig};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting NAINA OS Desktop Host (naina-desktop)...");

    let runtime = Arc::new(runtime::Runtime::with_default_kernel(
        runtime::RuntimeConfig,
    ));
    let services = Arc::new(services::ServiceRegistry::new(
        services::ServicesConfig,
        Arc::clone(&runtime),
    ));

    let config = DesktopHostConfig::default();
    let app = DesktopHostApp::boot(config, runtime, services)?;

    println!(
        "NAINA OS Desktop Host initialized in state: {:?}",
        app.state()
    );

    // Execute dry-run MVN turn
    let dummy_pcm = vec![0u8; 1600];
    let output_wav = app.process_voice_turn(&dummy_pcm)?;
    println!(
        "Processed MVN turn cleanly. Generated {} audio bytes.",
        output_wav.len()
    );

    app.shutdown()?;
    println!("NAINA OS Desktop Host shut down cleanly.");

    Ok(())
}
