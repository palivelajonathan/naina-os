use capabilities::{CapabilityConfig, CapabilityError, CapabilityRegistry};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_01_registry_construction() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    assert!(format!("{:?}", registry).contains("CapabilityRegistry"));
}

#[test]
fn test_02_token_granting() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("runtime", "CAP_DESKTOP_CONTROL", None)
        .unwrap();

    assert_eq!(token.capability_id(), "CAP_DESKTOP_CONTROL");
    assert_eq!(token.subject(), "runtime");
    assert!(!token.is_expired());
}

#[test]
fn test_03_deterministic_token_ids() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let t1 = registry.grant("s1", "CAP_READ", None).unwrap();
    let t2 = registry.grant("s2", "CAP_WRITE", None).unwrap();

    assert_eq!(t1.id(), "cap_tok_1");
    assert_eq!(t2.id(), "cap_tok_2");
}

#[test]
fn test_04_capability_id_preservation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("orchestrator", "CAP_MODEL_INFERENCE", None)
        .unwrap();

    assert_eq!(token.capability_id(), "CAP_MODEL_INFERENCE");
}

#[test]
fn test_05_subject_preservation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("browser_service", "CAP_CDP", None).unwrap();

    assert_eq!(token.subject(), "browser_service");
}

#[test]
fn test_06_token_expiration() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("short_lived", "CAP_TEMP", Some(Duration::from_millis(10)))
        .unwrap();

    thread::sleep(Duration::from_millis(20));
    assert!(token.is_expired());

    let res = registry.authorize(&token, "CAP_TEMP");
    assert!(matches!(res, Err(CapabilityError::TokenExpired { .. })));
}

#[test]
fn test_07_non_expiring_tokens() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("perm", "CAP_PERM", None).unwrap();

    assert!(!token.is_expired());
    assert!(registry.authorize(&token, "CAP_PERM").is_ok());
}

#[test]
fn test_08_successful_authorization() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("subsystem", "CAP_OBSIDIAN_READ", None)
        .unwrap();

    assert!(registry.authorize(&token, "CAP_OBSIDIAN_READ").is_ok());
}

#[test]
fn test_09_capability_mismatch() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry
        .grant("subsystem", "CAP_OBSIDIAN_READ", None)
        .unwrap();

    let res = registry.authorize(&token, "CAP_OBSIDIAN_WRITE");
    match res {
        Err(CapabilityError::Unauthorized { capability_id, .. }) => {
            assert_eq!(capability_id, "CAP_OBSIDIAN_WRITE");
        }
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_10_unknown_token() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let other_registry = CapabilityRegistry::new(CapabilityConfig);

    let token = other_registry.grant("sub", "CAP_TEST", None).unwrap();

    let res = registry.authorize(&token, "CAP_TEST");
    assert!(matches!(res, Err(CapabilityError::TokenNotFound { .. })));
}

#[test]
fn test_11_revocation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("sub", "CAP_TEST", None).unwrap();

    let revoked = registry.revoke(token.id()).unwrap();
    assert!(revoked);
}

#[test]
fn test_12_authorization_after_revocation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("sub", "CAP_TEST", None).unwrap();

    registry.revoke(token.id()).unwrap();

    let res = registry.authorize(&token, "CAP_TEST");
    assert!(matches!(res, Err(CapabilityError::TokenRevoked { .. })));
}

#[test]
fn test_13_repeated_revocation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("sub", "CAP_TEST", None).unwrap();

    assert!(registry.revoke(token.id()).unwrap());
    assert!(registry.revoke(token.id()).unwrap());
}

#[test]
fn test_14_concurrent_registry_access() {
    let registry = Arc::new(CapabilityRegistry::new(CapabilityConfig));
    let mut handles = Vec::new();

    for i in 0..10 {
        let reg = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            let token = reg
                .grant(format!("sub_{i}"), "CAP_CONCURRENT", None)
                .unwrap();
            reg.authorize(&token, "CAP_CONCURRENT").unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_15_arc_sharing() {
    let registry = Arc::new(CapabilityRegistry::new(CapabilityConfig));
    let token = registry.grant("sub", "CAP_SHARED", None).unwrap();

    let reg_clone = Arc::clone(&registry);
    let handle = thread::spawn(move || reg_clone.authorize(&token, "CAP_SHARED"));

    assert!(handle.join().unwrap().is_ok());
}

#[test]
fn test_16_deny_by_default_behavior() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let token = registry.grant("sub", "CAP_A", None).unwrap();

    assert!(registry.authorize(&token, "CAP_B").is_err());
}

#[test]
fn test_17_ttl_calculation() {
    let registry = CapabilityRegistry::new(CapabilityConfig);
    let ttl = Duration::from_secs(60);
    let token = registry.grant("sub", "CAP_TTL", Some(ttl)).unwrap();

    assert!(!token.is_expired());
}

#[test]
fn test_18_error_variants() {
    let err1 = CapabilityError::Unauthorized {
        capability_id: "CAP_X".to_string(),
        message: "denied".to_string(),
    };
    let err2 = CapabilityError::TokenExpired {
        token_id: "t1".to_string(),
    };
    let err3 = CapabilityError::TokenRevoked {
        token_id: "t2".to_string(),
    };
    let err4 = CapabilityError::TokenNotFound {
        token_id: "t3".to_string(),
    };

    assert!(err1.to_string().contains("CAP_X"));
    assert!(err2.to_string().contains("t1"));
    assert!(err3.to_string().contains("t2"));
    assert!(err4.to_string().contains("t3"));
}
