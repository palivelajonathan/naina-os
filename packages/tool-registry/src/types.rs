//! Types for the NAINA OS tool-registry package.

/// Unique identifier for a registered tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolId(pub u64);

/// Name identifier for a registered tool.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolName(pub String);

/// Lifecycle state of a registered tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolState {
    Registered,
    Active,
    Disabled,
    Failed,
}

/// Definition describing a tool's capabilities, description, and parameter schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: ToolName,
    pub description: String,
    pub parameters_schema: String,
}

/// Record representing a registered tool within the framework.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRecord {
    pub id: ToolId,
    pub definition: ToolDefinition,
    pub state: ToolState,
    pub execution_context_id: Option<runtime::ExecutionContextId>,
}
