use event_bus::{
    Event, EventBus, EventBusConfig, EventBusError, EventHandler, Result, SubscriptionId,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
struct TestHandler {
    id: usize,
    events: Arc<Mutex<Vec<Event>>>,
    call_order: Option<Arc<Mutex<Vec<usize>>>>,
    should_fail: bool,
}

impl TestHandler {
    fn new(id: usize) -> (Self, Arc<Mutex<Vec<Event>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                id,
                events: Arc::clone(&events),
                call_order: None,
                should_fail: false,
            },
            events,
        )
    }

    fn ordered(id: usize, order: Arc<Mutex<Vec<usize>>>) -> (Self, Arc<Mutex<Vec<Event>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                id,
                events: Arc::clone(&events),
                call_order: Some(order),
                should_fail: false,
            },
            events,
        )
    }

    fn failing(id: usize) -> (Self, Arc<Mutex<Vec<Event>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                id,
                events: Arc::clone(&events),
                call_order: None,
                should_fail: true,
            },
            events,
        )
    }
}

impl EventHandler for TestHandler {
    fn handle(&self, event: &Event) -> Result<()> {
        if self.should_fail {
            return Err(EventBusError::Handler {
                subscription_id: SubscriptionId(self.id as u64),
                message: format!("handler {id} failed", id = self.id),
            });
        }
        self.events.lock().unwrap().push(event.clone());
        if let Some(ref order) = self.call_order {
            order.lock().unwrap().push(self.id);
        }
        Ok(())
    }
}

#[test]
fn test_01_event_construction() {
    let event = Event::new("system.boot", "kernel");
    assert_eq!(event.event_type(), "system.boot");
    assert_eq!(event.source(), "kernel");
    assert!(event.id().starts_with("evt_"));
}

#[test]
fn test_02_event_payload_construction() {
    let mut payload = BTreeMap::new();
    payload.insert("status".to_string(), "ok".to_string());
    let event = Event::new("system.boot", "kernel").with_payload(payload.clone());
    assert_eq!(event.payload(), &payload);
}

#[test]
fn test_03_event_metadata_accessors() {
    let before = std::time::SystemTime::now();
    let event = Event::new("test.topic", "test.source");
    let after = std::time::SystemTime::now();

    assert!(event.timestamp() >= before);
    assert!(event.timestamp() <= after);
    assert_eq!(event.event_type(), "test.topic");
    assert_eq!(event.source(), "test.source");
}

#[test]
fn test_04_eventbus_construction() {
    let bus = EventBus::new(EventBusConfig);
    let event = Event::new("test", "src");
    assert!(bus.publish(&event).is_ok());
}

#[test]
fn test_05_subscribe_returns_unique_ids() {
    let bus = EventBus::new(EventBusConfig);
    let (h1, _) = TestHandler::new(1);
    let (h2, _) = TestHandler::new(2);

    let id1 = bus.subscribe("topic", Box::new(h1)).unwrap();
    let id2 = bus.subscribe("topic", Box::new(h2)).unwrap();

    assert_ne!(id1, id2);
}

#[test]
fn test_06_single_subscriber_receives_matching_event() {
    let bus = EventBus::new(EventBusConfig);
    let (handler, events) = TestHandler::new(1);
    bus.subscribe("topic.a", Box::new(handler)).unwrap();

    let event = Event::new("topic.a", "source.x");
    bus.publish(&event).unwrap();

    let rec = events.lock().unwrap();
    assert_eq!(rec.len(), 1);
    assert_eq!(rec[0].event_type(), "topic.a");
}

#[test]
fn test_07_subscriber_does_not_receive_unrelated_event() {
    let bus = EventBus::new(EventBusConfig);
    let (handler, events) = TestHandler::new(1);
    bus.subscribe("topic.a", Box::new(handler)).unwrap();

    let event = Event::new("topic.b", "source.x");
    bus.publish(&event).unwrap();

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn test_08_multiple_subscribers_receive_events() {
    let bus = EventBus::new(EventBusConfig);
    let (h1, e1) = TestHandler::new(1);
    let (h2, e2) = TestHandler::new(2);

    bus.subscribe("topic", Box::new(h1)).unwrap();
    bus.subscribe("topic", Box::new(h2)).unwrap();

    let event = Event::new("topic", "src");
    bus.publish(&event).unwrap();

    assert_eq!(e1.lock().unwrap().len(), 1);
    assert_eq!(e2.lock().unwrap().len(), 1);
}

#[test]
fn test_09_registration_order_is_preserved() {
    let bus = EventBus::new(EventBusConfig);
    let order = Arc::new(Mutex::new(Vec::new()));

    let (h1, _) = TestHandler::ordered(10, Arc::clone(&order));
    let (h2, _) = TestHandler::ordered(20, Arc::clone(&order));
    let (h3, _) = TestHandler::ordered(30, Arc::clone(&order));

    bus.subscribe("topic", Box::new(h1)).unwrap();
    bus.subscribe("topic", Box::new(h2)).unwrap();
    bus.subscribe("topic", Box::new(h3)).unwrap();

    let event = Event::new("topic", "src");
    bus.publish(&event).unwrap();

    assert_eq!(*order.lock().unwrap(), vec![10, 20, 30]);
}

#[test]
fn test_10_unsubscribe_removes_subscriber() {
    let bus = EventBus::new(EventBusConfig);
    let (h1, e1) = TestHandler::new(1);

    let sub_id = bus.subscribe("topic", Box::new(h1)).unwrap();
    let res = bus.unsubscribe(sub_id);

    assert!(res.unwrap());

    let event = Event::new("topic", "src");
    bus.publish(&event).unwrap();
    assert!(e1.lock().unwrap().is_empty());
}

#[test]
fn test_11_unsubscribe_unknown_id_returns_false() {
    let bus = EventBus::new(EventBusConfig);
    let res = bus.unsubscribe(SubscriptionId(9999));
    assert!(!res.unwrap());
}

#[test]
fn test_12_remaining_subscriber_order_survives_unsubscribe() {
    let bus = EventBus::new(EventBusConfig);
    let order = Arc::new(Mutex::new(Vec::new()));

    let (h1, _) = TestHandler::ordered(1, Arc::clone(&order));
    let (h2, _) = TestHandler::ordered(2, Arc::clone(&order));
    let (h3, _) = TestHandler::ordered(3, Arc::clone(&order));

    let _id1 = bus.subscribe("topic", Box::new(h1)).unwrap();
    let id2 = bus.subscribe("topic", Box::new(h2)).unwrap();
    let _id3 = bus.subscribe("topic", Box::new(h3)).unwrap();

    bus.unsubscribe(id2).unwrap();

    let event = Event::new("topic", "src");
    bus.publish(&event).unwrap();

    assert_eq!(*order.lock().unwrap(), vec![1, 3]);
}

#[test]
fn test_13_empty_bus_publish_succeeds() {
    let bus = EventBus::new(EventBusConfig);
    let event = Event::new("unhandled.topic", "src");
    assert!(bus.publish(&event).is_ok());
}

#[test]
fn test_14_handler_error_stops_dispatch() {
    let bus = EventBus::new(EventBusConfig);
    let (h_fail, _) = TestHandler::failing(1);
    let (h2, e2) = TestHandler::new(2);

    bus.subscribe("topic", Box::new(h_fail)).unwrap();
    bus.subscribe("topic", Box::new(h2)).unwrap();

    let event = Event::new("topic", "src");
    let res = bus.publish(&event);

    assert!(res.is_err());
    assert!(e2.lock().unwrap().is_empty());
}

#[test]
fn test_15_first_handler_error_is_returned() {
    let bus = EventBus::new(EventBusConfig);
    let (h_fail, _) = TestHandler::failing(42);

    bus.subscribe("topic", Box::new(h_fail)).unwrap();

    let event = Event::new("topic", "src");
    let res = bus.publish(&event);

    match res {
        Err(EventBusError::Handler {
            subscription_id, ..
        }) => {
            assert_eq!(subscription_id, SubscriptionId(42));
        }
        _ => panic!("Expected Handler error"),
    }
}

#[test]
fn test_16_later_handlers_are_not_invoked_after_failure() {
    let bus = EventBus::new(EventBusConfig);
    let order = Arc::new(Mutex::new(Vec::new()));

    let (h1, _) = TestHandler::ordered(1, Arc::clone(&order));
    let (h_fail, _) = TestHandler::failing(2);
    let (h3, _) = TestHandler::ordered(3, Arc::clone(&order));

    bus.subscribe("topic", Box::new(h1)).unwrap();
    bus.subscribe("topic", Box::new(h_fail)).unwrap();
    bus.subscribe("topic", Box::new(h3)).unwrap();

    let event = Event::new("topic", "src");
    let res = bus.publish(&event);

    assert!(res.is_err());
    assert_eq!(*order.lock().unwrap(), vec![1]);
}

#[test]
fn test_17_different_event_types_remain_isolated() {
    let bus = EventBus::new(EventBusConfig);
    let (h1, e1) = TestHandler::new(1);
    let (h2, e2) = TestHandler::new(2);

    bus.subscribe("topic.1", Box::new(h1)).unwrap();
    bus.subscribe("topic.2", Box::new(h2)).unwrap();

    let event1 = Event::new("topic.1", "src");
    bus.publish(&event1).unwrap();

    assert_eq!(e1.lock().unwrap().len(), 1);
    assert_eq!(e2.lock().unwrap().len(), 0);
}

#[test]
fn test_18_eventbus_can_be_shared_through_arc() {
    let bus = Arc::new(EventBus::new(EventBusConfig));
    let (handler, events) = TestHandler::new(1);

    let bus_clone = Arc::clone(&bus);
    let handle = thread::spawn(move || {
        bus_clone.subscribe("topic", Box::new(handler)).unwrap();
    });

    handle.join().unwrap();

    let event = Event::new("topic", "src");
    bus.publish(&event).unwrap();

    assert_eq!(events.lock().unwrap().len(), 1);
}

#[test]
fn test_19_concurrent_subscription_publication_does_not_panic() {
    let bus = Arc::new(EventBus::new(EventBusConfig));
    let mut handles = Vec::new();

    for i in 0..10 {
        let bus_clone = Arc::clone(&bus);
        handles.push(thread::spawn(move || {
            let (h, _) = TestHandler::new(i);
            let sub_id = bus_clone.subscribe("concurrent", Box::new(h)).unwrap();
            let event = Event::new("concurrent", "src");
            let _ = bus_clone.publish(&event);
            let _ = bus_clone.unsubscribe(sub_id);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_20_no_external_dependencies_beyond_logging() {
    let bus = EventBus::new(EventBusConfig);
    assert!(bus.publish(&Event::new("a", "b")).is_ok());
}
