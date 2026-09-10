# NAINA OS Configuration Package (`configuration`)

Production-grade configuration management subsystem for **NAINA OS**.

## Overview

The `configuration` package provides a unified, local-first configuration provider that loads system settings from TOML and JSON files, overrides settings with environment variables (`NAINA_*`), and validates resource parameters against official system specifications.

## Features

- **Multi-Format Support**: Parse `.toml` and `.json` configuration files seamlessly.
- **Environment Overrides**: Automatically override file settings with `NAINA_ENV`, `NAINA_HOST`, `NAINA_PORT`, `NAINA_LOG_LEVEL`, etc.
- **Validation**: Enforce resource limits and network boundary parameters before startup.
- **Production Safety**: Zero `unwrap()` calls in production paths with context-rich error reporting via `ConfigError`.

## Architecture & Public API

The package exposes two primary types:
- `ConfigLoader`: Fluent builder for loading and validating configuration.
- `ConfigProvider`: Trait providing read-only access to loaded system configuration (`Config`).

## Quickstart Example

```rust,ignore
use configuration::{ConfigLoader, ConfigProvider, Environment};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigLoader::new()
        .with_environment(Environment::Development)
        .load()?;

    println!("System Name: {}", config.get_config().name);
    println!("Server Port: {}", config.get_config().port);
    println!("Is Dev: {}", config.is_development());

    Ok(())
}
```

## Running Tests & Examples

```bash
cargo test -p configuration
cargo run -p configuration --example load_config
```
