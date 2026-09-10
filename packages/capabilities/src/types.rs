//! Types for the NAINA OS capabilities package.

use std::time::SystemTime;

/// A security token representing a capability permission grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityToken {
    pub(crate) id: String,
    pub(crate) capability_id: String,
    pub(crate) subject: String,
    pub(crate) issued_at: SystemTime,
    pub(crate) expires_at: Option<SystemTime>,
    pub(crate) revoked: bool,
}

impl CapabilityToken {
    /// Returns the unique identifier of the token.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the capability permission identifier (e.g. `CAP_DESKTOP_CONTROL`).
    pub fn capability_id(&self) -> &str {
        &self.capability_id
    }

    /// Returns the subject/subsystem to which the token was issued.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Checks whether the token has expired relative to current system time.
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }
}
