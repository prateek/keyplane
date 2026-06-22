//! Protocol Backend trait + Fake Backend (ADR 0015, 0033).

use keyplane_core::backend::{BackendUpdate, FakeBackend, ProtocolBackend};
use keyplane_core::health::{Capability, HealthState};
use keyplane_core::model::{LayerStack, StateConfidence};
use keyplane_core::provenance::SourceKind;

#[test]
fn fake_backend_declares_typed_capabilities() {
    let (backend, _model) = FakeBackend::demo();
    let descriptor = backend.descriptor();
    assert_eq!(descriptor.kind, SourceKind::Fake);
    assert!(descriptor.capabilities.has(Capability::StreamLayerStack));
    assert!(descriptor.capabilities.is_authoritative());
    assert!(backend.health().is_ok());
}

#[test]
fn poll_yields_one_scripted_update_at_a_time() {
    let (mut backend, _model) = FakeBackend::demo();
    let first = backend.poll();
    assert_eq!(first.len(), 1, "one step per poll animates the demo");
    assert!(matches!(first[0], BackendUpdate::LayerStack { .. }));
}

#[test]
fn replay_reloads_the_script() {
    let (mut backend, _model) = FakeBackend::demo();
    let mut count = 0;
    while !backend.poll().is_empty() {
        count += 1;
    }
    assert!(count >= 5);
    backend.replay();
    assert!(!backend.poll().is_empty(), "replay re-queues the script");
}

#[test]
fn health_changes_are_observable() {
    let mut backend = FakeBackend::new("fake", "Fake Backend");
    assert!(backend.health().is_ok());
    backend.set_health(HealthState::PermissionMissing {
        permission: "input-monitoring".into(),
        detail: "grant in System Settings".into(),
    });
    assert_eq!(backend.health().tag(), "permission-missing");
}

#[test]
fn enqueue_drives_custom_updates() {
    let mut backend = FakeBackend::new("fake", "Fake Backend");
    backend.enqueue(BackendUpdate::LayerStack {
        stack: LayerStack::base("layer-0"),
        confidence: StateConfidence::Inferred,
    });
    let updates = backend.poll();
    assert_eq!(updates.len(), 1);
}
