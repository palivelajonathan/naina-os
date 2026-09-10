//! Capability registry implementation for NAINA OS.

use crate::config::CapabilityConfig;
use crate::error::{CapabilityError, Result};
use crate::types::CapabilityToken;
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// Registry for managing Zero-Trust Capability-Based Access Control (CBAC) tokens.
#[derive(Debug)]
pub struct CapabilityRegistry {
    _config: CapabilityConfig,
    tokens: RwLock<BTreeMap<String, CapabilityToken>>,
    next_id: AtomicU64,
}

impl CapabilityRegistry {
    /// Creates a new [`CapabilityRegistry`] instance with the provided configuration.
    pub fn new(config: CapabilityConfig) -> Self {
        Self {
            _config: config,
            tokens: RwLock::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Grants a new [`CapabilityToken`] to a subject.
    ///
    /// Generates a deterministic token ID formatted as `cap_tok_<sequence>`.
    pub fn grant(
        &self,
        subject: impl Into<String>,
        capability_id: impl Into<String>,
        ttl: Option<Duration>,
    ) -> Result<CapabilityToken> {
        let seq = self.next_id.fetch_add(1, Ordering::SeqCst);
        let id = format!("cap_tok_{seq}");
        let issued_at = SystemTime::now();
        let expires_at = ttl.map(|d| issued_at + d);

        let token = CapabilityToken {
            id: id.clone(),
            capability_id: capability_id.into(),
            subject: subject.into(),
            issued_at,
            expires_at,
            revoked: false,
        };

        let mut map = self
            .tokens
            .write()
            .map_err(|e| CapabilityError::LockError {
                message: e.to_string(),
            })?;

        map.insert(id, token.clone());
        Ok(token)
    }

    /// Revokes a capability token by its ID.
    ///
    /// Returns `Ok(true)` if the token existed and was marked revoked.
    /// Returns `Err(CapabilityError::TokenNotFound)` if the token ID is unrecognized.
    pub fn revoke(&self, token_id: &str) -> Result<bool> {
        let mut map = self
            .tokens
            .write()
            .map_err(|e| CapabilityError::LockError {
                message: e.to_string(),
            })?;

        if let Some(token) = map.get_mut(token_id) {
            token.revoked = true;
            Ok(true)
        } else {
            Err(CapabilityError::TokenNotFound {
                token_id: token_id.to_string(),
            })
        }
    }

    /// Authorizes a presented token against a required capability ID.
    ///
    /// Authorization is deny-by-default and requires that:
    /// 1. The token exists in the registry.
    /// 2. The token is not marked revoked.
    /// 3. The token is not expired.
    /// 4. The token's `capability_id` matches `required_capability`.
    pub fn authorize(&self, token: &CapabilityToken, required_capability: &str) -> Result<()> {
        let map = self.tokens.read().map_err(|e| CapabilityError::LockError {
            message: e.to_string(),
        })?;

        let stored = map
            .get(token.id())
            .ok_or_else(|| CapabilityError::TokenNotFound {
                token_id: token.id().to_string(),
            })?;

        if stored.revoked {
            return Err(CapabilityError::TokenRevoked {
                token_id: token.id().to_string(),
            });
        }

        if stored.is_expired() {
            return Err(CapabilityError::TokenExpired {
                token_id: token.id().to_string(),
            });
        }

        if stored.capability_id() != required_capability {
            return Err(CapabilityError::Unauthorized {
                capability_id: required_capability.to_string(),
                message: format!(
                    "Token capability '{}' does not match required '{}'",
                    stored.capability_id(),
                    required_capability
                ),
            });
        }

        Ok(())
    }
}
