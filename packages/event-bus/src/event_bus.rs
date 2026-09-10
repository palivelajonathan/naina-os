//! System event bus implementation for NAINA OS.

use crate::config::EventBusConfig;
use crate::error::{EventBusError, Result};
use crate::event::Event;
use crate::traits::EventHandler;
use crate::types::SubscriptionId;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

struct Subscription {
    id: SubscriptionId,
    handler: Arc<dyn EventHandler>,
}

/// System event bus for publishing events and managing subscribers.
pub struct EventBus {
    _config: EventBusConfig,
    subscribers: RwLock<BTreeMap<String, Vec<Subscription>>>,
    next_id: AtomicU64,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("config", &self._config)
            .finish()
    }
}

impl EventBus {
    /// Creates a new [`EventBus`] instance with the provided configuration.
    pub fn new(config: EventBusConfig) -> Self {
        Self {
            _config: config,
            subscribers: RwLock::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Subscribes an [`EventHandler`] to events matching `event_type`.
    ///
    /// Subscriptions receive monotonically increasing [`SubscriptionId`]s.
    /// Handlers are called in subscription registration order.
    pub fn subscribe(
        &self,
        event_type: impl Into<String>,
        handler: Box<dyn EventHandler>,
    ) -> Result<SubscriptionId> {
        let event_type = event_type.into();
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::SeqCst));
        let subscription = Subscription {
            id,
            handler: Arc::from(handler),
        };

        let mut map = self
            .subscribers
            .write()
            .map_err(|e| EventBusError::LockError {
                message: e.to_string(),
            })?;

        map.entry(event_type).or_default().push(subscription);
        Ok(id)
    }

    /// Removes a subscription by its [`SubscriptionId`].
    ///
    /// Returns `Ok(true)` if the subscription was found and removed, or `Ok(false)` otherwise.
    pub fn unsubscribe(&self, id: SubscriptionId) -> Result<bool> {
        let mut map = self
            .subscribers
            .write()
            .map_err(|e| EventBusError::LockError {
                message: e.to_string(),
            })?;

        let mut removed = false;
        for subscribers in map.values_mut() {
            if let Some(pos) = subscribers.iter().position(|s| s.id == id) {
                subscribers.remove(pos);
                removed = true;
                break;
            }
        }

        map.retain(|_, v| !v.is_empty());
        Ok(removed)
    }

    /// Publishes an [`Event`] to all matching registered subscribers.
    ///
    /// Handlers are executed in registration order. If a handler returns an error:
    /// - Dispatch stops immediately
    /// - Remaining handlers are NOT invoked
    /// - The first error is returned from `publish()`
    pub fn publish(&self, event: &Event) -> Result<()> {
        let handlers = {
            let map = self
                .subscribers
                .read()
                .map_err(|e| EventBusError::LockError {
                    message: e.to_string(),
                })?;

            match map.get(event.event_type()) {
                Some(subs) => subs
                    .iter()
                    .map(|s| (s.id, Arc::clone(&s.handler)))
                    .collect::<Vec<_>>(),
                None => Vec::new(),
            }
        };

        for (_sub_id, handler) in handlers {
            handler.handle(event)?;
        }

        Ok(())
    }
}
