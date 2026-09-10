use configuration::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NAINA OS Configuration Example ===");

    let config = Config::default();

    println!("Runtime workers: {}", config.runtime.worker_threads);
    println!("Logging level: {:?}", config.logging.level);
    println!(
        "Unsafe operations allowed: {}",
        config.security.allow_unsafe_operations
    );
    println!("Configuration loaded successfully.");
    Ok(())
}
