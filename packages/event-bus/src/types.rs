//! Types for the NAINA OS event-bus package.

/// A unique identifier for a subscription on the event bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubscriptionId(pub u64);
