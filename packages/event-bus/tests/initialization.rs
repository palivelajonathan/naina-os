use event_bus::{EventBus, EventBusConfig};

#[test]
fn test_event_bus_instantiation() {
    let bus = EventBus::new(EventBusConfig);
    assert!(format!("{:?}", bus).contains("EventBus"));
}
