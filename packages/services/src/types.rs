//! Types for the NAINA OS services package.

/// Unique identifier for a registered service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceId(pub u64);

/// Name identifier for a registered service.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceName(pub String);

/// Lifecycle state of a registered service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Registered,
    Starting,
    Active,
    Degraded,
    Stopped,
    Failed,
}

/// Operational health state of a registered service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceHealth {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}

/// Record representing a registered user-space service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceRecord {
    pub id: ServiceId,
    pub name: ServiceName,
    pub state: ServiceState,
    pub health: ServiceHealth,
    pub execution_context_id: Option<runtime::ExecutionContextId>,
}
