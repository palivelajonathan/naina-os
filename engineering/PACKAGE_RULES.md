# Package Rules

This document defines the engineering standards for all packages in the repository.

## 1. Package naming

- Use lowercase, hyphenated names.
- Prefer component-oriented names over activity-oriented names.
- Reserve higher-level coordination packages such as orchestrator and tool-registry.

## 2. Allowed dependencies

- Dependencies must form a directed acyclic graph.
- Higher-level packages may depend on lower-level packages.
- Lower-level packages must not depend on higher-level packages.
- Avoid circular dependencies between runtime, kernel, services, and orchestrator.

## 3. Public API conventions

- Each package should expose a small, well-defined public API.
- Prefer traits, interfaces, and explicit types over implementation details.
- Public APIs should be documented in the package README.

## 4. Internal module layout

All Rust packages should follow the same internal layout:

- Cargo.toml
- README.md
- src/lib.rs
- src/error.rs
- src/types.rs
- src/traits.rs
- src/config.rs
- src/validation.rs
- tests/
- examples/

## 5. Testing requirements

- Every package must have unit tests for core behavior.
- Integration tests should be added for public-facing workflows.
- Tests must be runnable with cargo test.

## 6. Documentation requirements

- Every package must include a README.md.
- Public APIs should be documented with examples where helpful.
- Dependency changes must be reflected in engineering/DEPENDENCY_MAP.md.

## 7. Versioning policy

- Use semantic versioning.
- Increment the minor version for new public capabilities.
- Increment the major version for breaking API changes.
