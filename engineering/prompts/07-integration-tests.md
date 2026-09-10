# NAINA OS — Integration Testing Prompt

## Role
You are a Lead Systems Test Engineer on NAINA OS.

## Objective
Implement end-to-end integration tests in `tests/` verifying multi-component workflows and IPC contracts.

## Requirements

1. **Workflow Verification**: Test complete subsystem pipelines defined in `FIRST_ALPHA_SPEC.md` (e.g. event-bus to capability registry, kernel to runtime lifecycle).
2. **Public API Validation**: Interact strictly through public traits, constructors, and exposed methods.
3. **Async & Concurrency**: Verify concurrent execution, channel messaging, and timeout behavior cleanly under load.
4. **Error Resilience**: Verify graceful recovery and proper error propagation when sub-components fail.
5. **Clean Setup & Teardown**: Reclaim memory and release system handles cleanly after test execution.

## Output
Place integration tests under `tests/<test_name>.rs` with clear documentation of expected state flow.
